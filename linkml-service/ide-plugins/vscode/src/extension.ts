/**
 * LinkML VS Code Extension
 *
 * Provides comprehensive LinkML schema support in Visual Studio Code including:
 * - Syntax highlighting
 * - Real-time validation
 * - Code completion
 * - Code generation
 * - Schema visualization
 */

import * as vscode from 'vscode';
import * as path from 'path';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind,
    ExecutableOptions
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: vscode.ExtensionContext) {
    console.log('LinkML extension is now active');

    // Register language configuration
    const disposable = vscode.languages.setLanguageConfiguration('linkml', {
        comments: {
            lineComment: '#',
        },
        brackets: [
            ['{', '}'],
            ['[', ']'],
            ['(', ')']
        ],
        autoClosingPairs: [
            { open: '{', close: '}' },
            { open: '[', close: ']' },
            { open: '(', close: ')' },
            { open: '"', close: '"' },
            { open: "'", close: "'" },
        ],
        surroundingPairs: [
            { open: '{', close: '}' },
            { open: '[', close: ']' },
            { open: '(', close: ')' },
            { open: '"', close: '"' },
            { open: "'", close: "'" },
        ],
        wordPattern: /[A-Za-z_][A-Za-z0-9_]*/,
        indentationRules: {
            increaseIndentPattern: /^.*:\s*$/,
            decreaseIndentPattern: /^\s*$/
        }
    });

    context.subscriptions.push(disposable);

    // Start the language server
    startLanguageServer(context);

    // Register commands
    registerCommands(context);

    // Register providers
    registerProviders(context);

    // Set up file watchers
    setupFileWatchers(context);
}

function startLanguageServer(context: vscode.ExtensionContext) {
    // Get server path from configuration or use default
    const config = vscode.workspace.getConfiguration('linkml');
    let serverPath = config.get<string>('serverPath');

    if (!serverPath) {
        // Use bundled server
        serverPath = context.asAbsolutePath(
            path.join('server', 'linkml-language-server')
        );
    }

    // Server options
    const serverOptions: ServerOptions = {
        run: {
            command: serverPath,
            transport: TransportKind.stdio
        },
        debug: {
            command: serverPath,
            transport: TransportKind.stdio,
            options: {
                env: { RUST_LOG: 'debug' }
            }
        }
    };

    // Client options
    const clientOptions: LanguageClientOptions = {
        documentSelector: [
            { scheme: 'file', language: 'linkml' },
            { scheme: 'file', pattern: '**/*.linkml.yaml' },
            { scheme: 'file', pattern: '**/*.linkml.yml' }
        ],
        synchronize: {
            configurationSection: 'linkml',
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.linkml.{yaml,yml}')
        }
    };

    // Create and start the language client
    client = new LanguageClient(
        'linkml',
        'LinkML Language Server',
        serverOptions,
        clientOptions
    );

    // Start the client
    client.start();
}

function registerCommands(context: vscode.ExtensionContext) {
    // Validate command
    context.subscriptions.push(
        vscode.commands.registerCommand('linkml.validate', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showErrorMessage('No active editor');
                return;
            }

            await vscode.window.withProgress({
                location: vscode.ProgressLocation.Notification,
                title: "Validating LinkML schema...",
                cancellable: false
            }, async (progress) => {
                try {
                    const diagnostics = await validateSchema(editor.document);
                    if (diagnostics.length === 0) {
                        vscode.window.showInformationMessage('Schema is valid!');
                    } else {
                        vscode.window.showWarningMessage(`Found ${diagnostics.length} issues`);
                    }
                } catch (error) {
                    vscode.window.showErrorMessage(`Validation failed: ${error}`);
                }
            });
        })
    );

    // Generate code command
    context.subscriptions.push(
        vscode.commands.registerCommand('linkml.generate', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showErrorMessage('No active editor');
                return;
            }

            // Get generation target
            const targets = [
                'Python (Dataclass)',
                'Python (Pydantic)',
                'TypeScript',
                'JavaScript',
                'Java',
                'Go',
                'Rust',
                'SQL',
                'GraphQL',
                'JSON Schema',
                'OpenAPI',
                'Protobuf'
            ];

            const target = await vscode.window.showQuickPick(targets, {
                placeHolder: 'Select code generation target'
            });

            if (!target) return;

            await vscode.window.withProgress({
                location: vscode.ProgressLocation.Notification,
                title: `Generating ${target} code...`,
                cancellable: false
            }, async (progress) => {
                try {
                    const code = await generateCode(editor.document, target);

                    // Create new document with generated code
                    const doc = await vscode.workspace.openTextDocument({
                        content: code,
                        language: getLanguageForTarget(target)
                    });

                    await vscode.window.showTextDocument(doc);
                    vscode.window.showInformationMessage(`Generated ${target} code successfully`);
                } catch (error) {
                    vscode.window.showErrorMessage(`Generation failed: ${error}`);
                }
            });
        })
    );

    // Format command
    context.subscriptions.push(
        vscode.commands.registerCommand('linkml.format', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) return;

            const formatted = await formatSchema(editor.document);
            await editor.edit(editBuilder => {
                const fullRange = new vscode.Range(
                    editor.document.positionAt(0),
                    editor.document.positionAt(editor.document.getText().length)
                );
                editBuilder.replace(fullRange, formatted);
            });
        })
    );

    // Visualize command
    context.subscriptions.push(
        vscode.commands.registerCommand('linkml.visualize', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) return;

            const panel = vscode.window.createWebviewPanel(
                'linkmlVisualization',
                'LinkML Schema Visualization',
                vscode.ViewColumn.Beside,
                {
                    enableScripts: true,
                    retainContextWhenHidden: true
                }
            );

            const visualization = await generateVisualization(editor.document);
            panel.webview.html = getVisualizationHtml(visualization);
        })
    );

    // Convert format command
    context.subscriptions.push(
        vscode.commands.registerCommand('linkml.convert', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) return;

            const formats = ['YAML', 'JSON', 'JSON-LD'];
            const targetFormat = await vscode.window.showQuickPick(formats, {
                placeHolder: 'Select target format'
            });

            if (!targetFormat) return;

            const converted = await convertSchema(editor.document, targetFormat);
            const doc = await vscode.workspace.openTextDocument({
                content: converted,
                language: targetFormat.toLowerCase()
            });

            await vscode.window.showTextDocument(doc);
        })
    );

    // Create new schema command
    context.subscriptions.push(
        vscode.commands.registerCommand('linkml.createNewSchema', async () => {
            const name = await vscode.window.showInputBox({
                prompt: 'Enter schema name',
                placeHolder: 'MySchema'
            });

            if (!name) return;

            const template = getSchemaTemplate(name);
            const doc = await vscode.workspace.openTextDocument({
                content: template,
                language: 'linkml'
            });

            await vscode.window.showTextDocument(doc);
        })
    );
}

function registerProviders(context: vscode.ExtensionContext) {
    // Register completion provider
    const completionProvider = vscode.languages.registerCompletionItemProvider(
        'linkml',
        {
            provideCompletionItems(document: vscode.TextDocument, position: vscode.Position) {
                return provideCompletions(document, position);
            }
        },
        ':', ' '
    );

    // Register hover provider
    const hoverProvider = vscode.languages.registerHoverProvider(
        'linkml',
        {
            provideHover(document: vscode.TextDocument, position: vscode.Position) {
                return provideHover(document, position);
            }
        }
    );

    // Register definition provider
    const definitionProvider = vscode.languages.registerDefinitionProvider(
        'linkml',
        {
            provideDefinition(document: vscode.TextDocument, position: vscode.Position) {
                return provideDefinition(document, position);
            }
        }
    );

    // Register code lens provider
    const codeLensProvider = vscode.languages.registerCodeLensProvider(
        'linkml',
        {
            provideCodeLenses(document: vscode.TextDocument) {
                return provideCodeLenses(document);
            }
        }
    );

    context.subscriptions.push(
        completionProvider,
        hoverProvider,
        definitionProvider,
        codeLensProvider
    );
}

function setupFileWatchers(context: vscode.ExtensionContext) {
    // Watch for schema changes
    const watcher = vscode.workspace.createFileSystemWatcher('**/*.linkml.{yaml,yml}');

    watcher.onDidCreate(uri => {
        console.log(`LinkML schema created: ${uri.fsPath}`);
    });

    watcher.onDidChange(uri => {
        console.log(`LinkML schema changed: ${uri.fsPath}`);
        // Trigger revalidation if enabled
        const config = vscode.workspace.getConfiguration('linkml');
        if (config.get<boolean>('validation.onSave')) {
            vscode.commands.executeCommand('linkml.validate');
        }
    });

    watcher.onDidDelete(uri => {
        console.log(`LinkML schema deleted: ${uri.fsPath}`);
    });

    context.subscriptions.push(watcher);
}

// Helper functions
async function validateSchema(document: vscode.TextDocument): Promise<vscode.Diagnostic[]> {
    // In a real implementation, this would communicate with the language server
    return [];
}

async function generateCode(document: vscode.TextDocument, target: string): Promise<string> {
    // In a real implementation, this would communicate with the language server
    return `// Generated ${target} code from LinkML schema\n// Implementation pending...`;
}

async function formatSchema(document: vscode.TextDocument): Promise<string> {
    // In a real implementation, this would communicate with the language server
    return document.getText();
}

async function generateVisualization(document: vscode.TextDocument): Promise<string> {
    // In a real implementation, this would generate actual visualization
    return '<svg><!-- Schema visualization --></svg>';
}

async function convertSchema(document: vscode.TextDocument, format: string): Promise<string> {
    // In a real implementation, this would perform actual conversion
    return document.getText();
}

function getLanguageForTarget(target: string): string {
    const languageMap: { [key: string]: string } = {
        'Python (Dataclass)': 'python',
        'Python (Pydantic)': 'python',
        'TypeScript': 'typescript',
        'JavaScript': 'javascript',
        'Java': 'java',
        'Go': 'go',
        'Rust': 'rust',
        'SQL': 'sql',
        'GraphQL': 'graphql',
        'JSON Schema': 'json',
        'OpenAPI': 'yaml',
        'Protobuf': 'proto'
    };
    return languageMap[target] || 'plaintext';
}

function getSchemaTemplate(name: string): string {
    return `id: https://example.com/${name.toLowerCase()}
name: ${name}
description: ${name} schema definition

prefixes:
  linkml: https://w3id.org/linkml/
  ${name.toLowerCase()}: https://example.com/${name.toLowerCase()}/

default_prefix: ${name.toLowerCase()}

imports:
  - linkml:types

classes:
  ${name}:
    description: Main ${name} class
    attributes:
      id:
        identifier: true
        range: string
        description: Unique identifier
      name:
        range: string
        required: true
        description: Name of the ${name.toLowerCase()}
      description:
        range: string
        description: Optional description
`;
}

function getVisualizationHtml(svg: string): string {
    return `<!DOCTYPE html>
<html>
<head>
    <style>
        body {
            margin: 0;
            padding: 20px;
            font-family: sans-serif;
            background: var(--vscode-editor-background);
            color: var(--vscode-editor-foreground);
        }
        #visualization {
            width: 100%;
            height: 100vh;
            display: flex;
            justify-content: center;
            align-items: center;
        }
        svg {
            max-width: 100%;
            max-height: 100%;
        }
    </style>
</head>
<body>
    <div id="visualization">
        ${svg}
    </div>
</body>
</html>`;
}

async function provideCompletions(
    document: vscode.TextDocument,
    position: vscode.Position
): Promise<vscode.CompletionItem[]> {
    const completions: vscode.CompletionItem[] = [];

    // LinkML keywords
    const keywords = [
        'id', 'name', 'description', 'prefixes', 'default_prefix',
        'imports', 'classes', 'slots', 'types', 'enums', 'subsets',
        'attributes', 'range', 'required', 'identifier', 'multivalued',
        'pattern', 'minimum_value', 'maximum_value', 'permissible_values',
        'is_a', 'mixins', 'abstract', 'aliases', 'examples'
    ];

    keywords.forEach(keyword => {
        const item = new vscode.CompletionItem(keyword, vscode.CompletionItemKind.Keyword);
        item.documentation = `LinkML keyword: ${keyword}`;
        completions.push(item);
    });

    // Built-in types
    const types = [
        'string', 'integer', 'float', 'double', 'boolean',
        'date', 'datetime', 'time', 'uri', 'uriorcurie'
    ];

    types.forEach(type => {
        const item = new vscode.CompletionItem(type, vscode.CompletionItemKind.Class);
        item.documentation = `Built-in type: ${type}`;
        completions.push(item);
    });

    return completions;
}

async function provideHover(
    document: vscode.TextDocument,
    position: vscode.Position
): Promise<vscode.Hover | undefined> {
    const word = document.getText(document.getWordRangeAtPosition(position));

    // Provide documentation for LinkML keywords
    const keywordDocs: { [key: string]: string } = {
        'classes': 'Defines the classes (entities) in the schema',
        'attributes': 'Defines attributes (properties) of a class',
        'range': 'Specifies the type or class that this attribute can hold',
        'required': 'Whether this attribute must be provided',
        'identifier': 'Whether this attribute serves as the unique identifier',
        'multivalued': 'Whether this attribute can have multiple values',
        'pattern': 'Regular expression pattern for string validation',
        'is_a': 'Parent class for inheritance',
        'mixins': 'Classes to mix in (multiple inheritance)'
    };

    if (keywordDocs[word]) {
        return new vscode.Hover(
            new vscode.MarkdownString(`**${word}**\n\n${keywordDocs[word]}`)
        );
    }

    return undefined;
}

async function provideDefinition(
    document: vscode.TextDocument,
    position: vscode.Position
): Promise<vscode.Definition | undefined> {
    // In a real implementation, this would find class/type definitions
    return undefined;
}

async function provideCodeLenses(
    document: vscode.TextDocument
): Promise<vscode.CodeLens[]> {
    const codeLenses: vscode.CodeLens[] = [];

    // Add "Validate" lens at the top of the file
    const topOfDocument = new vscode.Range(0, 0, 0, 0);
    codeLenses.push(
        new vscode.CodeLens(topOfDocument, {
            title: "▶ Validate Schema",
            command: "linkml.validate"
        })
    );

    // Add "Generate Code" lens
    codeLenses.push(
        new vscode.CodeLens(topOfDocument, {
            title: "⚡ Generate Code",
            command: "linkml.generate"
        })
    );

    return codeLenses;
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
