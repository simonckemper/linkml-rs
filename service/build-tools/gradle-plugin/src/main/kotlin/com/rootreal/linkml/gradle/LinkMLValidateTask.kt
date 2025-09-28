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
 * Task to validate LinkML schemas.
 */
abstract class LinkMLValidateTask : DefaultTask() {

    @InputDirectory
    @Optional
    abstract val schemaDirectory: Property<File>

    @Input
    @Optional
    abstract val includes: ListProperty<String>

    @Input
    @Optional
    abstract val excludes: ListProperty<String>

    @Input
    abstract val linkmlExecutable: Property<String>

    @Input
    abstract val failOnError: Property<Boolean>

    @Input
    abstract val verbose: Property<Boolean>

    @Input
    @Optional
    abstract val validationOptions: ListProperty<String>

    @TaskAction
    fun validate() {
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

        var errorCount = 0
        schemaFiles.forEach { schemaFile ->
            try {
                validateSchema(schemaFile)
                logger.lifecycle("✓ Valid: ${project.relativePath(schemaFile)}")
            } catch (e: ValidationException) {
                errorCount++
                logger.error("✗ Invalid: ${project.relativePath(schemaFile)}")
                logger.error("  ${e.message}")
                if (verbose.get() && e.details != null) {
                    logger.error("  Details: ${e.details}")
                }
            }
        }

        if (errorCount > 0) {
            val message = "LinkML validation failed: $errorCount schema(s) with errors"
            if (failOnError.get()) {
                throw GradleException(message)
            } else {
                logger.warn(message)
            }
        } else {
            logger.lifecycle("All LinkML schemas are valid")
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

        // Add custom options
        validationOptions.getOrElse(emptyList()).forEach { option ->
            cmdLine.addArgument(option)
        }

        cmdLine.addArgument(schemaFile.absolutePath)

        val executor = DefaultExecutor()
        val outputStream = ByteArrayOutputStream()
        val errorStream = ByteArrayOutputStream()
        executor.streamHandler = PumpStreamHandler(outputStream, errorStream)

        val exitValue = executor.execute(cmdLine)

        if (exitValue != 0) {
            val output = outputStream.toString()
            val error = errorStream.toString()
            val details = if (error.isNotEmpty()) error else output
            throw ValidationException("Validation failed", details)
        }
    }

    private class ValidationException(message: String, val details: String?) : Exception(message)
}
