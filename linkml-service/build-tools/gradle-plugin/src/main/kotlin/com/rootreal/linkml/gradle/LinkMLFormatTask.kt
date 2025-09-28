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
 * Task to format LinkML schemas.
 */
abstract class LinkMLFormatTask : DefaultTask() {

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
    abstract val inPlace: Property<Boolean>

    @TaskAction
    fun format() {
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

        logger.lifecycle("Formatting ${schemaFiles.files.size} LinkML schema(s)")

        var successCount = 0
        var errorCount = 0

        schemaFiles.forEach { schemaFile ->
            try {
                formatSchema(schemaFile)
                successCount++
                logger.lifecycle("✓ Formatted: ${project.relativePath(schemaFile)}")
            } catch (e: Exception) {
                errorCount++
                logger.error("✗ Failed: ${project.relativePath(schemaFile)}")
                logger.error("  ${e.message}")
            }
        }

        logger.lifecycle("Formatting complete: $successCount succeeded, $errorCount failed")

        if (errorCount > 0) {
            throw GradleException("Formatting failed for $errorCount schema(s)")
        }
    }

    private fun findSchemaFiles(): FileTree {
        return project.fileTree(schemaDirectory.get()) {
            it.include(includes.get())
            it.exclude(excludes.get())
        }
    }

    private fun formatSchema(schemaFile: File) {
        // Build command
        val cmdLine = CommandLine(linkmlExecutable.get())
        cmdLine.addArgument("format")

        if (inPlace.get()) {
            cmdLine.addArgument("--in-place")
        }

        cmdLine.addArgument(schemaFile.absolutePath)

        // Execute command
        val executor = DefaultExecutor()
        val outputStream = ByteArrayOutputStream()
        val errorStream = ByteArrayOutputStream()
        executor.streamHandler = PumpStreamHandler(outputStream, errorStream)

        val exitValue = executor.execute(cmdLine)
        if (exitValue != 0) {
            throw GradleException("Formatting failed: ${errorStream.toString()}")
        }

        // If not in-place, write the formatted content back
        if (!inPlace.get()) {
            val formattedContent = outputStream.toString()
            schemaFile.writeText(formattedContent)
        }
    }
}
