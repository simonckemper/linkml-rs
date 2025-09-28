package com.rootreal.linkml.gradle

import org.gradle.api.Project
import org.gradle.api.provider.ListProperty
import org.gradle.api.provider.Property
import org.gradle.api.provider.MapProperty
import java.io.File

/**
 * Extension for configuring the LinkML plugin.
 */
open class LinkMLExtension(project: Project) {

    /**
     * Directory containing LinkML schema files.
     */
    val schemaDirectory: Property<File> = project.objects.property(File::class.java)
        .convention(project.layout.projectDirectory.dir("src/main/linkml").asFile)

    /**
     * Output directory for generated code.
     */
    val outputDirectory: Property<File> = project.objects.property(File::class.java)
        .convention(project.layout.buildDirectory.dir("generated/sources/linkml").get().asFile)

    /**
     * Code generator to use (e.g., "java", "python", "typescript").
     */
    val generator: Property<String> = project.objects.property(String::class.java)
        .convention("java")

    /**
     * Include patterns for schema files.
     */
    val includes: ListProperty<String> = project.objects.listProperty(String::class.java)
        .convention(listOf("**/*.linkml.yaml", "**/*.linkml.yml", "**/*.linkml"))

    /**
     * Exclude patterns for schema files.
     */
    val excludes: ListProperty<String> = project.objects.listProperty(String::class.java)
        .convention(emptyList())

    /**
     * Path to LinkML executable.
     */
    val executable: Property<String> = project.objects.property(String::class.java)
        .convention("linkml")

    /**
     * Package name for generated Java code.
     */
    val packageName: Property<String> = project.objects.property(String::class.java)

    /**
     * Whether to validate schemas before generating code.
     */
    val validateFirst: Property<Boolean> = project.objects.property(Boolean::class.java)
        .convention(true)

    /**
     * Whether to fail the build on validation errors.
     */
    val failOnError: Property<Boolean> = project.objects.property(Boolean::class.java)
        .convention(true)

    /**
     * Whether to automatically generate code during compilation.
     */
    val autoGenerate: Property<Boolean> = project.objects.property(Boolean::class.java)
        .convention(true)

    /**
     * Enable verbose output.
     */
    val verbose: Property<Boolean> = project.objects.property(Boolean::class.java)
        .convention(false)

    /**
     * Additional options for code generation.
     */
    val generationOptions: MapProperty<String, String> = project.objects.mapProperty(String::class.java, String::class.java)
        .convention(emptyMap())

    /**
     * Additional options for validation.
     */
    val validationOptions: ListProperty<String> = project.objects.listProperty(String::class.java)
        .convention(emptyList())
}
