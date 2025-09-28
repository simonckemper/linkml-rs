#!/usr/bin/env node

/**
 * LinkML CLI for npm package
 *
 * This provides a wrapper around the LinkML CLI with
 * additional JavaScript/TypeScript specific features.
 */

import { Command } from 'commander';
import * as fs from 'fs-extra';
import * as path from 'path';
import chalk from 'chalk';
import ora from 'ora';
import { LinkML, findSchemas, createDefaultSchema, saveSchema } from './index';

const program = new Command();

// Configure CLI
program
  .name('linkml')
  .description('LinkML schema validation and code generation')
  .version('2.0.0')
  .option('-v, --verbose', 'enable verbose output')
  .option('--executable <path>', 'path to LinkML executable', 'linkml');

// Validate command
program
  .command('validate <schema>')
  .description('Validate a LinkML schema')
  .option('-a, --all', 'validate all schemas in directory')
  .action(async (schemaPath: string, options) => {
    const spinner = ora('Validating schema...').start();

    try {
      const linkml = new LinkML({
        verbose: program.opts().verbose,
        executable: program.opts().executable
      });

      if (options.all) {
        // Validate all schemas in directory
        const schemas = await findSchemas(schemaPath);
        spinner.text = `Validating ${schemas.length} schemas...`;

        let errorCount = 0;
        for (const schema of schemas) {
          const result = await linkml.validate(schema);
          const relativePath = path.relative(process.cwd(), schema);

          if (result.valid) {
            console.log(chalk.green('✓'), relativePath);
          } else {
            console.log(chalk.red('✗'), relativePath);
            errorCount++;
            for (const error of result.errors) {
              console.log('  ', chalk.red(error.message));
            }
          }

          if (result.warnings.length > 0) {
            for (const warning of result.warnings) {
              console.log('  ', chalk.yellow('⚠'), warning.message);
            }
          }
        }

        spinner.stop();

        if (errorCount > 0) {
          console.log(chalk.red(`\n${errorCount} schema(s) failed validation`));
          process.exit(1);
        } else {
          console.log(chalk.green(`\nAll ${schemas.length} schemas are valid`));
        }
      } else {
        // Validate single schema
        const result = await linkml.validate(schemaPath);

        if (result.valid) {
          spinner.succeed('Schema is valid');

          if (result.warnings.length > 0) {
            console.log(chalk.yellow('\nWarnings:'));
            for (const warning of result.warnings) {
              console.log('  ', warning.message);
            }
          }
        } else {
          spinner.fail('Schema validation failed');

          console.log(chalk.red('\nErrors:'));
          for (const error of result.errors) {
            console.log('  ', error.message);
          }

          process.exit(1);
        }
      }
    } catch (error: any) {
      spinner.fail(error.message);
      process.exit(1);
    }
  });

// Generate command
program
  .command('generate <schema>')
  .description('Generate code from LinkML schema')
  .requiredOption('-t, --target <language>', 'target language (typescript, javascript, etc.)')
  .requiredOption('-o, --output <path>', 'output directory or file')
  .option('--package <name>', 'package name')
  .option('--no-validate', 'skip validation before generation')
  .action(async (schemaPath: string, options) => {
    const spinner = ora('Generating code...').start();

    try {
      const linkml = new LinkML({
        verbose: program.opts().verbose,
        executable: program.opts().executable
      });

      // Validate first if requested
      if (options.validate !== false) {
        spinner.text = 'Validating schema...';
        const result = await linkml.validate(schemaPath);

        if (!result.valid) {
          spinner.fail('Schema validation failed');
          for (const error of result.errors) {
            console.log('  ', chalk.red(error.message));
          }
          process.exit(1);
        }
      }

      // Generate code
      spinner.text = `Generating ${options.target} code...`;

      await linkml.generate(schemaPath, {
        target: options.target,
        output: options.output,
        packageName: options.package
      });

      spinner.succeed(`Generated ${options.target} code to ${options.output}`);

      // Special handling for TypeScript/JavaScript
      if (options.target === 'typescript' || options.target === 'javascript') {
        console.log(chalk.blue('\nNext steps:'));
        console.log('  1. Install dependencies: npm install');
        console.log('  2. Import generated types in your code');
        console.log('  3. Use the types for type-safe data handling');
      }

    } catch (error: any) {
      spinner.fail(error.message);
      process.exit(1);
    }
  });

// Convert command
program
  .command('convert <schema>')
  .description('Convert schema to another format')
  .requiredOption('-f, --format <format>', 'target format (json, jsonld, rdf, ttl)')
  .requiredOption('-o, --output <path>', 'output file')
  .action(async (schemaPath: string, options) => {
    const spinner = ora('Converting schema...').start();

    try {
      const linkml = new LinkML({
        verbose: program.opts().verbose,
        executable: program.opts().executable
      });

      await linkml.convert(schemaPath, {
        format: options.format,
        output: options.output
      });

      spinner.succeed(`Converted to ${options.format} format: ${options.output}`);

    } catch (error: any) {
      spinner.fail(error.message);
      process.exit(1);
    }
  });

// Format command
program
  .command('format <schema>')
  .description('Format LinkML schema')
  .option('--check', 'check if formatting is needed without modifying')
  .option('--no-in-place', 'output to stdout instead of modifying file')
  .action(async (schemaPath: string, options) => {
    const spinner = ora('Formatting schema...').start();

    try {
      const linkml = new LinkML({
        verbose: program.opts().verbose,
        executable: program.opts().executable
      });

      if (options.check) {
        // Just check formatting
        const formatted = await linkml.format(schemaPath, false);
        const original = await fs.readFile(schemaPath, 'utf8');

        if (formatted === original) {
          spinner.succeed('Schema is properly formatted');
        } else {
          spinner.fail('Schema needs formatting');
          process.exit(1);
        }
      } else {
        await linkml.format(schemaPath, options.inPlace !== false);
        spinner.succeed('Schema formatted');
      }

    } catch (error: any) {
      spinner.fail(error.message);
      process.exit(1);
    }
  });

// Info command
program
  .command('info <schema>')
  .description('Display schema information')
  .action(async (schemaPath: string) => {
    try {
      const linkml = new LinkML({
        verbose: program.opts().verbose,
        executable: program.opts().executable
      });

      const info = await linkml.getSchemaInfo(schemaPath);

      console.log(chalk.bold('\nSchema Information:'));
      console.log('  ID:', info.id || chalk.gray('(not specified)'));
      console.log('  Name:', info.name || chalk.gray('(not specified)'));
      console.log('  Version:', info.version || chalk.gray('(not specified)'));

      if (info.description) {
        console.log('  Description:', info.description);
      }

      console.log('\n' + chalk.bold('Contents:'));
      console.log('  Classes:', info.classes.length);
      if (info.classes.length > 0) {
        console.log('    ', info.classes.slice(0, 5).join(', ') +
                    (info.classes.length > 5 ? `, ... (${info.classes.length - 5} more)` : ''));
      }

      console.log('  Slots:', info.slots.length);
      if (info.slots.length > 0) {
        console.log('    ', info.slots.slice(0, 5).join(', ') +
                    (info.slots.length > 5 ? `, ... (${info.slots.length - 5} more)` : ''));
      }

      console.log('  Types:', info.types.length);
      console.log('  Enums:', info.enums.length);

    } catch (error: any) {
      console.error(chalk.red('Error:'), error.message);
      process.exit(1);
    }
  });

// Init command
program
  .command('init')
  .description('Initialize a new LinkML project')
  .option('-n, --name <name>', 'schema name', 'MySchema')
  .option('-d, --dir <directory>', 'project directory', '.')
  .option('-t, --typescript', 'set up for TypeScript')
  .action(async (options) => {
    const spinner = ora('Initializing LinkML project...').start();

    try {
      const projectDir = path.resolve(options.dir);
      const schemaDir = path.join(projectDir, 'schemas');
      const schemaFile = path.join(schemaDir, `${options.name.toLowerCase()}.linkml.yaml`);

      // Create directories
      await fs.ensureDir(schemaDir);

      // Create schema
      const schema = createDefaultSchema(options.name);
      await saveSchema(schema, schemaFile);

      // Create package.json if it doesn't exist
      const packageJsonPath = path.join(projectDir, 'package.json');
      if (!await fs.pathExists(packageJsonPath)) {
        const packageJson = {
          name: options.name.toLowerCase(),
          version: '0.1.0',
          description: `${options.name} LinkML schema project`,
          scripts: {
            'validate': 'linkml validate schemas/',
            'generate': `linkml generate schemas/${options.name.toLowerCase()}.linkml.yaml -t ${options.typescript ? 'typescript' : 'javascript'} -o src/generated/`,
            'format': 'linkml format schemas/'
          },
          devDependencies: {
            '@rootreal/linkml': '^2.0.0'
          }
        };

        if (options.typescript) {
          packageJson.devDependencies['typescript'] = '^5.3.3';
          packageJson.devDependencies['@types/node'] = '^20.10.5';
        }

        await fs.writeJson(packageJsonPath, packageJson, { spaces: 2 });
      }

      // Create .gitignore
      const gitignorePath = path.join(projectDir, '.gitignore');
      if (!await fs.pathExists(gitignorePath)) {
        const gitignore = [
          'node_modules/',
          'dist/',
          'src/generated/',
          '*.log'
        ].join('\n');

        await fs.writeFile(gitignorePath, gitignore);
      }

      // Create tsconfig.json if TypeScript
      if (options.typescript) {
        const tsconfigPath = path.join(projectDir, 'tsconfig.json');
        if (!await fs.pathExists(tsconfigPath)) {
          const tsconfig = {
            compilerOptions: {
              target: 'ES2022',
              module: 'commonjs',
              lib: ['ES2022'],
              outDir: './dist',
              rootDir: './src',
              strict: true,
              esModuleInterop: true,
              skipLibCheck: true,
              forceConsistentCasingInFileNames: true,
              resolveJsonModule: true,
              declaration: true,
              declarationMap: true
            },
            include: ['src/**/*'],
            exclude: ['node_modules', 'dist']
          };

          await fs.writeJson(tsconfigPath, tsconfig, { spaces: 2 });
        }
      }

      spinner.succeed('LinkML project initialized');

      console.log(chalk.green('\nProject structure created:'));
      console.log('  schemas/');
      console.log(`    ${options.name.toLowerCase()}.linkml.yaml`);
      console.log('  package.json');
      console.log('  .gitignore');
      if (options.typescript) {
        console.log('  tsconfig.json');
      }

      console.log(chalk.blue('\nNext steps:'));
      console.log('  1. cd', options.dir);
      console.log('  2. npm install');
      console.log('  3. npm run validate');
      console.log('  4. npm run generate');

    } catch (error: any) {
      spinner.fail(error.message);
      process.exit(1);
    }
  });

// Parse arguments
program.parse(process.argv);

// Show help if no command
if (!process.argv.slice(2).length) {
  program.outputHelp();
}
