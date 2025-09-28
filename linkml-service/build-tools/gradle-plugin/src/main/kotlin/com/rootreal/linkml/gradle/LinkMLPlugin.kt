package com.rootreal.linkml.gradle

import org.gradle.api.Plugin
import org.gradle.api.Project
import org.gradle.api.plugins.JavaPlugin
import org.gradle.api.plugins.JavaPluginExtension
import org.gradle.api.tasks.SourceSetContainer
import java.io.File

/**
 * LinkML Gradle Plugin for schema validation and code generation.
 */
class LinkMLPlugin : Plugin<Project> {

    override fun apply(project: Project) {
        // Create extension
        val extension = project.extensions.create("linkml", LinkMLExtension::class.java, project)

        // Register tasks
        registerValidateTask(project, extension)
        registerGenerateTask(project, extension)
        registerConvertTask(project, extension)
        registerFormatTask(project, extension)

        // Configure source sets for Java projects
        project.plugins.withType(JavaPlugin::class.java) {
            configureJavaProject(project, extension)
        }

        // Add default task dependencies
        project.afterEvaluate {
            configureTaskDependencies(project, extension)
        }
    }

    private fun registerValidateTask(project: Project, extension: LinkMLExtension) {
        project.tasks.register("linkmlValidate", LinkMLValidateTask::class.java) { task ->
            task.group = "linkml"
            task.description = "Validate LinkML schemas"
            task.schemaDirectory.set(extension.schemaDirectory)
            task.includes.set(extension.includes)
            task.excludes.set(extension.excludes)
            task.linkmlExecutable.set(extension.executable)
            task.failOnError.set(extension.failOnError)
            task.verbose.set(extension.verbose)
        }
    }

    private fun registerGenerateTask(project: Project, extension: LinkMLExtension) {
        project.tasks.register("linkmlGenerate", LinkMLGenerateTask::class.java) { task ->
            task.group = "linkml"
            task.description = "Generate code from LinkML schemas"
            task.schemaDirectory.set(extension.schemaDirectory)
            task.outputDirectory.set(extension.outputDirectory)
            task.generator.set(extension.generator)
            task.includes.set(extension.includes)
            task.excludes.set(extension.excludes)
            task.linkmlExecutable.set(extension.executable)
            task.packageName.set(extension.packageName)
            task.validateFirst.set(extension.validateFirst)
            task.verbose.set(extension.verbose)

            // Make generate task depend on validate if enabled
            if (extension.validateFirst.get()) {
                task.dependsOn("linkmlValidate")
            }
        }
    }

    private fun registerConvertTask(project: Project, extension: LinkMLExtension) {
        project.tasks.register("linkmlConvert", LinkMLConvertTask::class.java) { task ->
            task.group = "linkml"
            task.description = "Convert LinkML schemas to other formats"
            task.schemaDirectory.set(extension.schemaDirectory)
            task.outputDirectory.set(extension.outputDirectory)
            task.targetFormat.set("json")
            task.includes.set(extension.includes)
            task.excludes.set(extension.excludes)
            task.linkmlExecutable.set(extension.executable)
        }
    }

    private fun registerFormatTask(project: Project, extension: LinkMLExtension) {
        project.tasks.register("linkmlFormat", LinkMLFormatTask::class.java) { task ->
            task.group = "linkml"
            task.description = "Format LinkML schemas"
            task.schemaDirectory.set(extension.schemaDirectory)
            task.includes.set(extension.includes)
            task.excludes.set(extension.excludes)
            task.linkmlExecutable.set(extension.executable)
            task.inPlace.set(true)
        }
    }

    private fun configureJavaProject(project: Project, extension: LinkMLExtension) {
        val javaExtension = project.extensions.getByType(JavaPluginExtension::class.java)
        val sourceSets = javaExtension.sourceSets

        // Add generated sources to main source set
        sourceSets.getByName("main").java {
            srcDir(extension.outputDirectory)
        }
    }

    private fun configureTaskDependencies(project: Project, extension: LinkMLExtension) {
        // Make compile depend on generate for Java projects
        if (project.plugins.hasPlugin(JavaPlugin::class.java) && extension.autoGenerate.get()) {
            project.tasks.named("compileJava") {
                it.dependsOn("linkmlGenerate")
            }
        }

        // Make check depend on validate
        project.tasks.findByName("check")?.dependsOn("linkmlValidate")
    }
}
