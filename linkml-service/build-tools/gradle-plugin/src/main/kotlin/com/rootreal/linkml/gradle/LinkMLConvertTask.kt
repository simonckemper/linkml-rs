package com.rootreal.linkml.gradle

import org.gradle.api.DefaultTask
import org.gradle.api.GradleException
import org.gradle.api.file.FileTree
import org.gradle.api.provider.ListProperty
import org.gradle.api.provider.Property
import org.gradle.api.tasks.*
import org.apache.commons.exec.CommandLine
import org.apache.commons.exec.DefaultExecutor
import org.apache.commons.exec.PumpStreamHandler
import java.io.ByteArrayOutputStream
import java.io.File

/**
 * Task to convert LinkML schemas to other formats.
 */
abstract class LinkMLConvertTask : DefaultTask() {

    @InputDirectory
    @Optional
    abstract val schemaDirectory: Property<File>

    @OutputDirectory
    abstract val outputDirectory: Property<File>

    @Input
    abstract val targetFormat: Property<String>

    @Input
    @Optional
    abstract val includes: ListProperty<String>

    @Input
    @Optional
    abstract val excludes: ListProperty<String>

    @Input
    abstract val linkmlExecutable: Property<String>

    @TaskAction
    fun convert() {
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

        logger.lifecycle("Converting ${schemaFiles.files.size} schema(s) to ${targetFormat.get()} format")

        // Create output directory
        val outDir = outputDirectory.get()
        if (!outDir.exists()) {
            outDir.mkdirs()
        }

        var successCount = 0
        var errorCount = 0

        schemaFiles.forEach { schemaFile ->
            try {
                val outputFile = convertSchema(schemaFile)
                successCount++
                logger.lifecycle("✓ Converted: ${project.relativePath(outputFile)}")
            } catch (e: Exception) {
                errorCount++
                logger.error("✗ Failed: ${project.relativePath(schemaFile)}")
                logger.error("  ${e.message}")
            }
        }

        logger.lifecycle("Conversion complete: $successCount succeeded, $errorCount failed")

        if (errorCount > 0) {
            throw GradleException("Conversion failed for $errorCount schema(s)")
        }
    }

    private fun findSchemaFiles(): FileTree {
        return project.fileTree(schemaDirectory.get()) {
            it.include(includes.get())
            it.exclude(excludes.get())
        }
    }

    private fun convertSchema(schemaFile: File): File {
        // Determine output file
        val baseName = schemaFile.nameWithoutExtension
        val outputFile = File(outputDirectory.get(), "$baseName.${targetFormat.get()}")

        // Build command
        val cmdLine = CommandLine(linkmlExecutable.get())
        cmdLine.addArgument("convert")
        cmdLine.addArgument("-f")
        cmdLine.addArgument(targetFormat.get())
        cmdLine.addArgument("-o")
        cmdLine.addArgument(outputFile.absolutePath)
        cmdLine.addArgument(schemaFile.absolutePath)

        // Execute command
        val executor = DefaultExecutor()
        val outputStream = ByteArrayOutputStream()
        val errorStream = ByteArrayOutputStream()
        executor.streamHandler = PumpStreamHandler(outputStream, errorStream)

        val exitValue = executor.execute(cmdLine)
        if (exitValue != 0) {
            throw GradleException("Conversion failed: ${errorStream.toString()}")
        }

        return outputFile
    }
}
