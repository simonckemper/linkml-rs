/**
 * LinkML JavaScript/TypeScript API
 *
 * This module provides programmatic access to LinkML functionality
 * for JavaScript and TypeScript projects.
 */

import { execSync } from 'child_process';
import * as fs from 'fs-extra';
import * as path from 'path';
import * as yaml from 'js-yaml';
import { glob } from 'glob';
import which from 'which';

export interface LinkMLOptions {
  /** Path to LinkML executable */
  executable?: string;
  /** Enable verbose output */
  verbose?: boolean;
  /** Working directory */
  cwd?: string;
}

export interface ValidationResult {
  valid: boolean;
  errors: ValidationError[];
  warnings: ValidationWarning[];
}

export interface ValidationError {
  message: string;
  line?: number;
  column?: number;
  path?: string;
}

export interface ValidationWarning {
  message: string;
  line?: number;
  column?: number;
  path?: string;
}

export interface GenerationOptions {
  /** Target language/format */
  target: string;
  /** Output directory or file */
  output: string;
  /** Package name (for Java/Python) */
  packageName?: string;
  /** Additional generator options */
  options?: Record<string, any>;
}

export interface ConversionOptions {
  /** Target format (json, jsonld, rdf, ttl) */
  format: string;
  /** Output file */
  output: string;
}

/**
 * LinkML client for programmatic access
 */
export class LinkML {
  private executable: string;
  private verbose: boolean;
  private cwd?: string;

  constructor(options: LinkMLOptions = {}) {
    this.executable = options.executable || 'linkml';
    this.verbose = options.verbose || false;
    this.cwd = options.cwd;

    // Check if LinkML is available
    this.checkExecutable();
  }

  /**
   * Check if LinkML executable is available
   */
  private checkExecutable(): void {
    try {
      which.sync(this.executable);
    } catch (error) {
      throw new Error(`LinkML executable not found: ${this.executable}. Please install LinkML first.`);
    }
  }

  /**
   * Execute a LinkML command
   */
  private execute(args: string[]): string {
    const command = `${this.executable} ${args.join(' ')}`;

    try {
      if (this.verbose) {
        console.log(`Executing: ${command}`);
      }

      const output = execSync(command, {
        encoding: 'utf8',
        cwd: this.cwd,
        stdio: ['pipe', 'pipe', 'pipe']
      });

      return output;
    } catch (error: any) {
      if (error.stderr) {
        throw new Error(error.stderr.toString());
      }
      throw error;
    }
  }

  /**
   * Validate a LinkML schema
   */
  async validate(schemaPath: string): Promise<ValidationResult> {
    const args = ['validate'];

    if (this.verbose) {
      args.push('--verbose');
    }

    args.push(schemaPath);

    try {
      const output = this.execute(args);

      // Parse validation results
      return this.parseValidationOutput(output);
    } catch (error: any) {
      // Parse error output
      return this.parseValidationError(error.message);
    }
  }

  /**
   * Validate multiple schemas
   */
  async validateAll(pattern: string): Promise<Map<string, ValidationResult>> {
    const files = await glob(pattern);
    const results = new Map<string, ValidationResult>();

    for (const file of files) {
      const result = await this.validate(file);
      results.set(file, result);
    }

    return results;
  }

  /**
   * Generate code from a schema
   */
  async generate(schemaPath: string, options: GenerationOptions): Promise<void> {
    const args = ['generate', '-t', options.target, '-o', options.output];

    if (options.packageName) {
      args.push('--package', options.packageName);
    }

    if (options.options) {
      for (const [key, value] of Object.entries(options.options)) {
        args.push(`--${key}`);
        if (value !== true) {
          args.push(String(value));
        }
      }
    }

    args.push(schemaPath);

    this.execute(args);
  }

  /**
   * Convert a schema to another format
   */
  async convert(schemaPath: string, options: ConversionOptions): Promise<void> {
    const args = ['convert', '-f', options.format, '-o', options.output, schemaPath];
    this.execute(args);
  }

  /**
   * Format a schema
   */
  async format(schemaPath: string, inPlace: boolean = true): Promise<string> {
    const args = ['format'];

    if (inPlace) {
      args.push('--in-place');
    }

    args.push(schemaPath);

    return this.execute(args);
  }

  /**
   * Load and parse a LinkML schema
   */
  async loadSchema(schemaPath: string): Promise<any> {
    const content = await fs.readFile(schemaPath, 'utf8');
    return yaml.load(content);
  }

  /**
   * Get schema information
   */
  async getSchemaInfo(schemaPath: string): Promise<SchemaInfo> {
    const schema = await this.loadSchema(schemaPath);

    return {
      id: schema.id,
      name: schema.name,
      description: schema.description,
      version: schema.version,
      classes: Object.keys(schema.classes || {}),
      slots: Object.keys(schema.slots || {}),
      types: Object.keys(schema.types || {}),
      enums: Object.keys(schema.enums || {})
    };
  }

  /**
   * Parse validation output
   */
  private parseValidationOutput(output: string): ValidationResult {
    // Simple parsing - in reality would be more sophisticated
    const errors: ValidationError[] = [];
    const warnings: ValidationWarning[] = [];

    const lines = output.split('\n');
    for (const line of lines) {
      if (line.includes('ERROR')) {
        errors.push({ message: line });
      } else if (line.includes('WARNING')) {
        warnings.push({ message: line });
      }
    }

    return {
      valid: errors.length === 0,
      errors,
      warnings
    };
  }

  /**
   * Parse validation error
   */
  private parseValidationError(error: string): ValidationResult {
    return {
      valid: false,
      errors: [{ message: error }],
      warnings: []
    };
  }
}

export interface SchemaInfo {
  id?: string;
  name?: string;
  description?: string;
  version?: string;
  classes: string[];
  slots: string[];
  types: string[];
  enums: string[];
}

/**
 * Utility functions
 */

/**
 * Find all LinkML schemas in a directory
 */
export async function findSchemas(dir: string): Promise<string[]> {
  const patterns = [
    path.join(dir, '**/*.linkml.yaml'),
    path.join(dir, '**/*.linkml.yml'),
    path.join(dir, '**/*.linkml')
  ];

  const files: string[] = [];
  for (const pattern of patterns) {
    const matches = await glob(pattern);
    files.push(...matches);
  }

  return [...new Set(files)].sort();
}

/**
 * Create a default LinkML schema
 */
export function createDefaultSchema(name: string): any {
  return {
    id: `https://example.com/${name.toLowerCase()}`,
    name: name,
    description: `${name} schema definition`,
    version: '0.1.0',
    prefixes: {
      linkml: 'https://w3id.org/linkml/',
      [name.toLowerCase()]: `https://example.com/${name.toLowerCase()}/`
    },
    default_prefix: name.toLowerCase(),
    imports: ['linkml:types'],
    classes: {
      [name]: {
        description: `Main ${name} class`,
        attributes: {
          id: {
            identifier: true,
            range: 'string',
            description: 'Unique identifier'
          },
          name: {
            range: 'string',
            required: true,
            description: `Name of the ${name.toLowerCase()}`
          }
        }
      }
    }
  };
}

/**
 * Save a schema to file
 */
export async function saveSchema(schema: any, filePath: string): Promise<void> {
  const content = yaml.dump(schema, {
    indent: 2,
    lineWidth: -1,
    noRefs: true,
    sortKeys: false
  });

  await fs.writeFile(filePath, content, 'utf8');
}

// Export default instance
export default LinkML;
