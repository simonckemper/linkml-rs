# @rootreal/linkml

LinkML schema validation and code generation for JavaScript/TypeScript projects.

## Installation

```bash
npm install -D @rootreal/linkml
# or
yarn add -D @rootreal/linkml
# or
pnpm add -D @rootreal/linkml
```

## Prerequisites

This package requires the LinkML CLI to be installed:

```bash
# Install LinkML (requires Python)
pip install linkml
```

## Usage

### CLI Usage

```bash
# Validate a schema
npx linkml validate schema.linkml.yaml

# Generate TypeScript code
npx linkml generate schema.linkml.yaml -t typescript -o src/generated/

# Convert to JSON Schema
npx linkml convert schema.linkml.yaml -f jsonschema -o schema.json

# Format a schema
npx linkml format schema.linkml.yaml

# Initialize a new project
npx linkml init --name MyProject --typescript
```

### Programmatic Usage

```typescript
import { LinkML } from '@rootreal/linkml';

const linkml = new LinkML({
  verbose: true
});

// Validate a schema
const result = await linkml.validate('schema.linkml.yaml');
if (result.valid) {
  console.log('Schema is valid!');
} else {
  console.error('Validation errors:', result.errors);
}

// Generate code
await linkml.generate('schema.linkml.yaml', {
  target: 'typescript',
  output: 'src/generated/'
});

// Convert format
await linkml.convert('schema.linkml.yaml', {
  format: 'jsonschema',
  output: 'schema.json'
});

// Load and inspect schema
const info = await linkml.getSchemaInfo('schema.linkml.yaml');
console.log('Classes:', info.classes);
console.log('Slots:', info.slots);
```

## Project Setup

### Initialize a New Project

```bash
npx linkml init --name MyProject --typescript
```

This creates:
- `schemas/` - Directory for LinkML schemas
- `package.json` - With LinkML scripts
- `tsconfig.json` - TypeScript configuration (if --typescript)
- `.gitignore` - Git ignore file

### Package.json Scripts

Add these scripts to your `package.json`:

```json
{
  "scripts": {
    "validate": "linkml validate schemas/",
    "generate": "linkml generate schemas/*.linkml.yaml -t typescript -o src/generated/",
    "format": "linkml format schemas/",
    "build": "npm run validate && npm run generate && tsc"
  }
}
```

## API Reference

### LinkML Class

#### Constructor Options

```typescript
interface LinkMLOptions {
  executable?: string;  // Path to LinkML CLI (default: 'linkml')
  verbose?: boolean;    // Enable verbose output
  cwd?: string;        // Working directory
}
```

#### Methods

##### validate(schemaPath: string): Promise<ValidationResult>

Validates a LinkML schema.

```typescript
const result = await linkml.validate('schema.linkml.yaml');
```

##### validateAll(pattern: string): Promise<Map<string, ValidationResult>>

Validates multiple schemas matching a glob pattern.

```typescript
const results = await linkml.validateAll('schemas/**/*.linkml.yaml');
```

##### generate(schemaPath: string, options: GenerationOptions): Promise<void>

Generates code from a schema.

```typescript
await linkml.generate('schema.linkml.yaml', {
  target: 'typescript',
  output: 'src/generated/',
  packageName: 'com.example'
});
```

##### convert(schemaPath: string, options: ConversionOptions): Promise<void>

Converts a schema to another format.

```typescript
await linkml.convert('schema.linkml.yaml', {
  format: 'jsonschema',
  output: 'schema.json'
});
```

##### format(schemaPath: string, inPlace?: boolean): Promise<string>

Formats a schema.

```typescript
// Format in place
await linkml.format('schema.linkml.yaml');

// Get formatted content
const formatted = await linkml.format('schema.linkml.yaml', false);
```

##### getSchemaInfo(schemaPath: string): Promise<SchemaInfo>

Gets information about a schema.

```typescript
const info = await linkml.getSchemaInfo('schema.linkml.yaml');
console.log('Classes:', info.classes);
```

### Utility Functions

#### findSchemas(dir: string): Promise<string[]>

Finds all LinkML schemas in a directory.

```typescript
import { findSchemas } from '@rootreal/linkml';

const schemas = await findSchemas('schemas/');
```

#### createDefaultSchema(name: string): any

Creates a default schema object.

```typescript
import { createDefaultSchema, saveSchema } from '@rootreal/linkml';

const schema = createDefaultSchema('Person');
await saveSchema(schema, 'person.linkml.yaml');
```

## Examples

### Basic Schema Validation

```typescript
import { LinkML } from '@rootreal/linkml';

async function validateSchemas() {
  const linkml = new LinkML();
  
  const schemas = await linkml.validateAll('schemas/**/*.linkml.yaml');
  
  for (const [file, result] of schemas) {
    if (result.valid) {
      console.log(`✓ ${file}`);
    } else {
      console.log(`✗ ${file}`);
      result.errors.forEach(error => {
        console.log(`  - ${error.message}`);
      });
    }
  }
}
```

### TypeScript Code Generation

```typescript
import { LinkML } from '@rootreal/linkml';
import * as path from 'path';

async function generateTypes() {
  const linkml = new LinkML();
  
  const schemas = ['person.linkml.yaml', 'organization.linkml.yaml'];
  
  for (const schema of schemas) {
    await linkml.generate(path.join('schemas', schema), {
      target: 'typescript',
      output: 'src/generated/'
    });
  }
}
```

### Schema Creation

```typescript
import { createDefaultSchema, saveSchema } from '@rootreal/linkml';

async function createPersonSchema() {
  const schema = createDefaultSchema('Person');
  
  // Add custom attributes
  schema.classes.Person.attributes.email = {
    range: 'string',
    pattern: '^\\S+@\\S+\\.\\S+$',
    description: 'Email address'
  };
  
  schema.classes.Person.attributes.age = {
    range: 'integer',
    minimum_value: 0,
    maximum_value: 150,
    description: 'Age in years'
  };
  
  await saveSchema(schema, 'schemas/person.linkml.yaml');
}
```

## TypeScript Integration

When generating TypeScript code, the plugin creates fully typed interfaces:

```typescript
// Generated code example
export interface Person {
  id: string;
  name: string;
  email?: string;
  age?: number;
}
```

Use the generated types in your application:

```typescript
import { Person } from './generated/person';

const person: Person = {
  id: '123',
  name: 'John Doe',
  email: 'john@example.com',
  age: 30
};
```

## Contributing

See the main [RootReal repository](https://github.com/simonckemper/rootreal) for contribution guidelines.

## License

MIT
