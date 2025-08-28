package com.rootreal.linkml.gradle

import org.gradle.api.DefaultTask
import org.gradle.api.GradleException
import org.gradle.api.file.FileTree
import org.gradle.api.provider.ListProperty
import org.gradle.api.provider.Property
import org.gradle.api.provider.MapProperty
import org.gradle.api.tasks.*
import org.apache.commons.exec.CommandLine
import org.apache.commons.exec.DefaultExecutor
import org.apache.commons.exec.PumpStreamHandler
import org.apache.commons.io.FileUtils
import java.io.ByteArrayOutputStream
import java.io.File

/**
 * Task to generate code from LinkML schemas.
 */
abstract class LinkMLGenerateTask : DefaultTask() {

    @InputDirectory
    @Optional
    abstract val schemaDirectory: Property<File>

    @OutputDirectory
    abstract val outputDirectory: Property<File>

    @Input
    abstract val generator: Property<String>

    @Input
    @Optional
    abstract val includes: ListProperty<String>

    @Input
    @Optional
    abstract val excludes: ListProperty<String>

    @Input
    abstract val linkmlExecutable: Property<String>

    @Input
    @Optional
    abstract val packageName: Property<String>

    @Input
    abstract val validateFirst: Property<Boolean>

    @Input
    abstract val verbose: Property<Boolean>

    @Input
    @Optional
    abstract val generationOptions: MapProperty<String, String>

    @TaskAction
    fun generate() {
        val schemaDir = schemaDirectory.get()
        if (!schemaDir.exists()) {
            logger.lifecycle("Schema directory does not exist: $schemaDir")
            return
        }

        val schemaFiles = findSchemaFiles()
        if (schemaFiles.isEmpty) {
            logger.lifecycle("No LinkML schema files found")
            return
        }

        logger.lifecycle("Found ${schemaFiles.files.size} LinkML schema file(s)")
        logger.lifecycle("Generating ${generator.get()} code to ${outputDirectory.get()}")

        // Create output directory
        val outDir = outputDirectory.get()
        if (!outDir.exists()) {
            outDir.mkdirs()
        }

        var successCount = 0
        var errorCount = 0

        schemaFiles.forEach { schemaFile ->
            try {
                // Validate first if requested
                if (validateFirst.get()) {
                    validateSchema(schemaFile)
                }

                // Generate code
                val outputFile = generateCode(schemaFile)
                successCount++
                logger.lifecycle("✓ Generated: ${project.relativePath(outputFile)}")

            } catch (e: Exception) {
                errorCount++
                logger.error("✗ Failed: ${project.relativePath(schemaFile)}")
                logger.error("  ${e.message}")
                if (verbose.get() && e.cause != null) {
                    logger.error("  Cause: ${e.cause?.message}")
                }
            }
        }

        logger.lifecycle("Code generation complete: $successCount succeeded, $errorCount failed")

        if (errorCount > 0) {
            throw GradleException("Code generation failed for $errorCount schema(s)")
        }
    }

    private fun findSchemaFiles(): FileTree {
        return project.fileTree(schemaDirectory.get()) {
            it.include(includes.get())
            it.exclude(excludes.get())
        }
    }

    private fun validateSchema(schemaFile: File) {
        val cmdLine = CommandLine(linkmlExecutable.get())
        cmdLine.addArgument("validate")
        cmdLine.addArgument(schemaFile.absolutePath)

        val executor = DefaultExecutor()
        val errorStream = ByteArrayOutputStream()
        executor.streamHandler = PumpStreamHandler(null, errorStream)

        val exitValue = executor.execute(cmdLine)
        if (exitValue != 0) {
            throw GradleException("Schema validation failed: ${errorStream.toString()}")
        }
    }

    private fun generateCode(schemaFile: File): File {
        // Determine output file
        var baseName = schemaFile.name
        when {
            baseName.endsWith(".linkml.yaml") -> baseName = baseName.substring(0, baseName.length - 12)
            baseName.endsWith(".linkml.yml") -> baseName = baseName.substring(0, baseName.length - 11)
            baseName.endsWith(".linkml") -> baseName = baseName.substring(0, baseName.length - 7)
        }

        val extension = getFileExtension(generator.get())
        val outputFile = File(outputDirectory.get(), "$baseName.$extension")

        // Build command
        val cmdLine = CommandLine(linkmlExecutable.get())
        cmdLine.addArgument("generate")
        cmdLine.addArgument("-t")
        cmdLine.addArgument(generator.get())
        cmdLine.addArgument("-o")
        cmdLine.addArgument(outputFile.absolutePath)

        // Add package name for Java
        if (generator.get() == "java" && packageName.isPresent) {
            cmdLine.addArgument("--package")
            cmdLine.addArgument(packageName.get())
        }

        // Add custom options
        generationOptions.getOrElse(emptyMap()).forEach { (key, value) ->
            cmdLine.addArgument("--$key")
            if (value.isNotEmpty()) {
                cmdLine.addArgument(value)
            }
        }

        cmdLine.addArgument(schemaFile.absolutePath)

        // Execute command
        val executor = DefaultExecutor()
        val outputStream = ByteArrayOutputStream()
        val errorStream = ByteArrayOutputStream()
        executor.streamHandler = PumpStreamHandler(outputStream, errorStream)

        val exitValue = executor.execute(cmdLine)
        if (exitValue != 0) {
            throw GradleException("Code generation failed: ${errorStream.toString()}")
        }

        // Handle Java package structure
        if (generator.get() == "java" && packageName.isPresent) {
            organizeJavaPackage(outputFile)
        }

        return outputFile
    }

    private fun getFileExtension(generator: String): String {
        return when (generator) {
            "python", "pydantic" -> "py"
            "typescript" -> "ts"
            "javascript" -> "js"
            "java" -> "java"
            "go" -> "go"
            "rust" -> "rs"
            "sql" -> "sql"
            "graphql" -> "graphql"
            "jsonschema" -> "json"
            "shacl" -> "ttl"
            "owl" -> "owl"
            else -> "txt"
        }
    }

    private fun organizeJavaPackage(generatedFile: File) {
        val pkgName = packageName.orNull ?: return
        if (pkgName.isEmpty()) return

        // Create package directory structure
        val packagePath = pkgName.replace('.', File.separatorChar)
        val packageDir = File(outputDirectory.get(), packagePath)
        packageDir.mkdirs()

        // Move file to package directory
        val targetFile = File(packageDir, generatedFile.name)
        if (generatedFile.exists()) {
            FileUtils.moveFile(generatedFile, targetFile)
        }
    }
}
