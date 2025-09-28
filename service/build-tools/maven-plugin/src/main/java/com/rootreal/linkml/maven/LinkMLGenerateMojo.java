package com.rootreal.linkml.maven;

import org.apache.maven.plugin.AbstractMojo;
import org.apache.maven.plugin.MojoExecutionException;
import org.apache.maven.plugin.MojoFailureException;
import org.apache.maven.plugins.annotations.LifecyclePhase;
import org.apache.maven.plugins.annotations.Mojo;
import org.apache.maven.plugins.annotations.Parameter;
import org.apache.maven.project.MavenProject;
import org.apache.commons.exec.CommandLine;
import org.apache.commons.exec.DefaultExecutor;
import org.apache.commons.exec.ExecuteException;
import org.apache.commons.exec.PumpStreamHandler;
import org.apache.commons.io.FileUtils;
import org.codehaus.plexus.util.DirectoryScanner;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.IOException;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.HashMap;

/**
 * Maven goal to generate code from LinkML schemas.
 */
@Mojo(name = "generate", defaultPhase = LifecyclePhase.GENERATE_SOURCES, threadSafe = true)
public class LinkMLGenerateMojo extends AbstractMojo {

    /**
     * The Maven project.
     */
    @Parameter(defaultValue = "${project}", readonly = true, required = true)
    private MavenProject project;

    /**
     * Location of LinkML schema files.
     */
    @Parameter(defaultValue = "${project.basedir}/src/main/linkml", property = "linkml.schemaDirectory")
    private File schemaDirectory;

    /**
     * Output directory for generated code.
     */
    @Parameter(defaultValue = "${project.build.directory}/generated-sources/linkml", property = "linkml.outputDirectory")
    private File outputDirectory;

    /**
     * Target language for code generation.
     */
    @Parameter(defaultValue = "java", property = "linkml.generator")
    private String generator;

    /**
     * Include patterns for schema files.
     */
    @Parameter(property = "linkml.includes")
    private String[] includes;

    /**
     * Exclude patterns for schema files.
     */
    @Parameter(property = "linkml.excludes")
    private String[] excludes;

    /**
     * Path to the LinkML executable.
     */
    @Parameter(defaultValue = "linkml", property = "linkml.executable")
    private String linkmlExecutable;

    /**
     * Skip code generation.
     */
    @Parameter(defaultValue = "false", property = "linkml.skip")
    private boolean skip;

    /**
     * Package name for generated Java code.
     */
    @Parameter(property = "linkml.packageName")
    private String packageName;

    /**
     * Additional generation options.
     */
    @Parameter(property = "linkml.generationOptions")
    private Map<String, String> generationOptions;

    /**
     * Validate schema before generation.
     */
    @Parameter(defaultValue = "true", property = "linkml.validateFirst")
    private boolean validateFirst;

    /**
     * Add generated sources to compile source root.
     */
    @Parameter(defaultValue = "true", property = "linkml.addCompileSourceRoot")
    private boolean addCompileSourceRoot;

    /**
     * Enable verbose output.
     */
    @Parameter(defaultValue = "false", property = "linkml.verbose")
    private boolean verbose;

    @Override
    public void execute() throws MojoExecutionException, MojoFailureException {
        if (skip) {
            getLog().info("Skipping LinkML code generation");
            return;
        }

        if (!schemaDirectory.exists()) {
            getLog().info("Schema directory does not exist: " + schemaDirectory);
            return;
        }

        // Find schema files
        List<File> schemaFiles = findSchemaFiles();
        if (schemaFiles.isEmpty()) {
            getLog().info("No LinkML schema files found");
            return;
        }

        getLog().info("Found " + schemaFiles.size() + " LinkML schema file(s)");
        getLog().info("Generating " + generator + " code to " + outputDirectory);

        // Create output directory
        if (!outputDirectory.exists()) {
            outputDirectory.mkdirs();
        }

        // Process each schema
        int successCount = 0;
        int errorCount = 0;

        for (File schemaFile : schemaFiles) {
            try {
                // Validate first if requested
                if (validateFirst) {
                    validateSchema(schemaFile);
                }

                // Generate code
                File outputFile = generateCode(schemaFile);
                successCount++;
                getLog().info("✓ Generated: " + getRelativePath(outputFile));

            } catch (Exception e) {
                errorCount++;
                getLog().error("✗ Failed: " + getRelativePath(schemaFile));
                getLog().error("  " + e.getMessage());
                if (verbose && e.getCause() != null) {
                    getLog().error("  Cause: " + e.getCause().getMessage());
                }
            }
        }

        // Add to compile source root if Java
        if (addCompileSourceRoot && "java".equals(generator)) {
            project.addCompileSourceRoot(outputDirectory.getAbsolutePath());
            getLog().info("Added generated sources to compile source root");
        }

        // Report results
        getLog().info("Code generation complete: " + successCount + " succeeded, " + errorCount + " failed");

        if (errorCount > 0) {
            throw new MojoFailureException("Code generation failed for " + errorCount + " schema(s)");
        }
    }

    /**
     * Find schema files based on includes/excludes patterns.
     */
    private List<File> findSchemaFiles() {
        DirectoryScanner scanner = new DirectoryScanner();
        scanner.setBasedir(schemaDirectory);

        // Set includes
        if (includes != null && includes.length > 0) {
            scanner.setIncludes(includes);
        } else {
            scanner.setIncludes(new String[]{
                "**/*.linkml.yaml",
                "**/*.linkml.yml",
                "**/*.linkml"
            });
        }

        // Set excludes
        if (excludes != null && excludes.length > 0) {
            scanner.setExcludes(excludes);
        }

        scanner.scan();

        List<File> files = new ArrayList<>();
        for (String filename : scanner.getIncludedFiles()) {
            files.add(new File(schemaDirectory, filename));
        }

        return files;
    }

    /**
     * Validate a schema file.
     */
    private void validateSchema(File schemaFile) throws MojoExecutionException {
        try {
            CommandLine cmdLine = new CommandLine(linkmlExecutable);
            cmdLine.addArgument("validate");
            cmdLine.addArgument(schemaFile.getAbsolutePath());

            DefaultExecutor executor = new DefaultExecutor();
            ByteArrayOutputStream errorStream = new ByteArrayOutputStream();
            executor.setStreamHandler(new PumpStreamHandler(null, errorStream));

            try {
                executor.execute(cmdLine);
            } catch (ExecuteException e) {
                throw new MojoExecutionException("Schema validation failed: " + errorStream.toString());
            }

        } catch (IOException e) {
            throw new MojoExecutionException("Failed to validate schema", e);
        }
    }

    /**
     * Generate code from a schema file.
     */
    private File generateCode(File schemaFile) throws MojoExecutionException {
        try {
            // Determine output file
            String baseName = schemaFile.getName();
            if (baseName.endsWith(".linkml.yaml")) {
                baseName = baseName.substring(0, baseName.length() - 12);
            } else if (baseName.endsWith(".linkml.yml")) {
                baseName = baseName.substring(0, baseName.length() - 11);
            } else if (baseName.endsWith(".linkml")) {
                baseName = baseName.substring(0, baseName.length() - 7);
            }

            // Determine file extension based on generator
            String extension = getFileExtension(generator);
            File outputFile = new File(outputDirectory, baseName + "." + extension);

            // Build command
            CommandLine cmdLine = new CommandLine(linkmlExecutable);
            cmdLine.addArgument("generate");
            cmdLine.addArgument("-t");
            cmdLine.addArgument(generator);
            cmdLine.addArgument("-o");
            cmdLine.addArgument(outputFile.getAbsolutePath());

            // Add package name for Java
            if ("java".equals(generator) && packageName != null) {
                cmdLine.addArgument("--package");
                cmdLine.addArgument(packageName);
            }

            // Add custom options
            if (generationOptions != null) {
                for (Map.Entry<String, String> entry : generationOptions.entrySet()) {
                    cmdLine.addArgument("--" + entry.getKey());
                    if (entry.getValue() != null && !entry.getValue().isEmpty()) {
                        cmdLine.addArgument(entry.getValue());
                    }
                }
            }

            cmdLine.addArgument(schemaFile.getAbsolutePath());

            // Execute command
            DefaultExecutor executor = new DefaultExecutor();
            ByteArrayOutputStream outputStream = new ByteArrayOutputStream();
            ByteArrayOutputStream errorStream = new ByteArrayOutputStream();
            PumpStreamHandler streamHandler = new PumpStreamHandler(outputStream, errorStream);
            executor.setStreamHandler(streamHandler);

            try {
                executor.execute(cmdLine);

                // Handle Java package structure
                if ("java".equals(generator) && packageName != null) {
                    organizeJavaPackage(outputFile);
                }

                return outputFile;

            } catch (ExecuteException e) {
                String error = errorStream.toString();
                throw new MojoExecutionException("Code generation failed: " + error);
            }

        } catch (IOException e) {
            throw new MojoExecutionException("Failed to generate code", e);
        }
    }

    /**
     * Get file extension for generator.
     */
    private String getFileExtension(String generator) {
        Map<String, String> extensions = new HashMap<>();
        extensions.put("python", "py");
        extensions.put("pydantic", "py");
        extensions.put("typescript", "ts");
        extensions.put("javascript", "js");
        extensions.put("java", "java");
        extensions.put("go", "go");
        extensions.put("rust", "rs");
        extensions.put("sql", "sql");
        extensions.put("graphql", "graphql");
        extensions.put("jsonschema", "json");
        extensions.put("shacl", "ttl");
        extensions.put("owl", "owl");

        return extensions.getOrDefault(generator, "txt");
    }

    /**
     * Organize generated Java file into package structure.
     */
    private void organizeJavaPackage(File generatedFile) throws IOException {
        if (packageName == null || packageName.isEmpty()) {
            return;
        }

        // Create package directory structure
        String packagePath = packageName.replace('.', File.separatorChar);
        File packageDir = new File(outputDirectory, packagePath);
        packageDir.mkdirs();

        // Move file to package directory
        File targetFile = new File(packageDir, generatedFile.getName());
        if (generatedFile.exists()) {
            FileUtils.moveFile(generatedFile, targetFile);
        }
    }

    /**
     * Get relative path for display.
     */
    private String getRelativePath(File file) {
        try {
            return project.getBasedir().toPath().relativize(file.toPath()).toString();
        } catch (Exception e) {
            return file.getAbsolutePath();
        }
    }
}
