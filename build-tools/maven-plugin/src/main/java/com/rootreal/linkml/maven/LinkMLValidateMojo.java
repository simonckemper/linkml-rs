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

/**
 * Maven goal to validate LinkML schemas.
 */
@Mojo(name = "validate", defaultPhase = LifecyclePhase.VALIDATE, threadSafe = true)
public class LinkMLValidateMojo extends AbstractMojo {

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
     * Skip validation.
     */
    @Parameter(defaultValue = "false", property = "linkml.skip")
    private boolean skip;

    /**
     * Fail build on validation errors.
     */
    @Parameter(defaultValue = "true", property = "linkml.failOnError")
    private boolean failOnError;

    /**
     * Enable verbose output.
     */
    @Parameter(defaultValue = "false", property = "linkml.verbose")
    private boolean verbose;

    /**
     * Additional validation options.
     */
    @Parameter(property = "linkml.validationOptions")
    private List<String> validationOptions;

    @Override
    public void execute() throws MojoExecutionException, MojoFailureException {
        if (skip) {
            getLog().info("Skipping LinkML validation");
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

        // Validate each schema
        int errorCount = 0;
        for (File schemaFile : schemaFiles) {
            try {
                validateSchema(schemaFile);
                getLog().info("✓ Valid: " + getRelativePath(schemaFile));
            } catch (ValidationException e) {
                errorCount++;
                getLog().error("✗ Invalid: " + getRelativePath(schemaFile));
                getLog().error("  " + e.getMessage());
                if (verbose && e.getDetails() != null) {
                    getLog().error("  Details: " + e.getDetails());
                }
            }
        }

        // Report results
        if (errorCount > 0) {
            String message = "LinkML validation failed: " + errorCount + " schema(s) with errors";
            if (failOnError) {
                throw new MojoFailureException(message);
            } else {
                getLog().warn(message);
            }
        } else {
            getLog().info("All LinkML schemas are valid");
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
     * Validate a single schema file.
     */
    private void validateSchema(File schemaFile) throws ValidationException {
        try {
            // Build command
            CommandLine cmdLine = new CommandLine(linkmlExecutable);
            cmdLine.addArgument("validate");

            // Add custom options
            if (validationOptions != null) {
                for (String option : validationOptions) {
                    cmdLine.addArgument(option);
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
            } catch (ExecuteException e) {
                String output = outputStream.toString();
                String error = errorStream.toString();
                String details = error.isEmpty() ? output : error;
                throw new ValidationException("Validation failed", details);
            }

        } catch (IOException e) {
            throw new ValidationException("Failed to execute LinkML validator: " + e.getMessage(), null);
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

    /**
     * Validation exception with details.
     */
    private static class ValidationException extends Exception {
        private final String details;

        public ValidationException(String message, String details) {
            super(message);
            this.details = details;
        }

        public String getDetails() {
            return details;
        }
    }
}
