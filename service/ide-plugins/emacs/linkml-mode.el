;;; linkml-mode.el --- Major mode for editing LinkML schemas -*- lexical-binding: t; -*-

;; Copyright (C) 2025 RootReal

;; Author: RootReal Team
;; Version: 2.0.0
;; Package-Requires: ((emacs "26.1") (yaml-mode "0.0.15") (flycheck "32") (company "0.9.13") (lsp-mode "8.0.0"))
;; Keywords: languages, data, yaml, linkml, schema
;; URL: https://github.com/simonckemper/rootreal

;;; Commentary:

;; This package provides a major mode for editing LinkML (Linked Data Modeling Language)
;; schema files in Emacs.  It includes syntax highlighting, validation, code completion,
;; and integration with the LinkML language server.

;; Features:
;; - Syntax highlighting for LinkML constructs
;; - Real-time validation via flycheck
;; - Code completion via company-mode
;; - LSP support for advanced features
;; - Code generation commands
;; - Schema visualization
;; - Imenu support for navigation

;;; Code:

(require 'yaml-mode)
(require 'flycheck)
(require 'company)
(require 'lsp-mode)
(require 'imenu)
(require 'easymenu)

(defgroup linkml nil
  "LinkML schema support."
  :group 'languages
  :prefix "linkml-")

(defcustom linkml-executable "linkml"
  "Path to the LinkML executable."
  :type 'string
  :group 'linkml)

(defcustom linkml-indent-offset 2
  "Indentation offset for LinkML schemas."
  :type 'integer
  :group 'linkml)

(defcustom linkml-validate-on-save t
  "Whether to validate LinkML schemas on save."
  :type 'boolean
  :group 'linkml)

(defcustom linkml-default-generation-target "python"
  "Default code generation target."
  :type '(choice (const "python")
                 (const "typescript")
                 (const "java")
                 (const "go")
                 (const "rust")
                 (const "sql")
                 (const "graphql")
                 (const "jsonschema"))
  :group 'linkml)

;;; Syntax highlighting

(defconst linkml-keywords
  '("id" "name" "title" "description" "version" "license"
    "prefixes" "default_prefix" "imports"
    "classes" "slots" "types" "enums" "subsets"
    "is_a" "mixins" "abstract" "mixin" "attributes"
    "range" "required" "identifier" "multivalued"
    "pattern" "minimum_value" "maximum_value"
    "permissible_values" "slot_usage" "aliases"
    "exact_mappings" "close_mappings" "mappings"
    "examples" "see_also" "deprecated" "comments"
    "domain" "slot_uri" "key" "designates_type"
    "equals_string" "equals_number" "minimum_cardinality"
    "maximum_cardinality" "exactly_one_of" "any_of"
    "all_of" "none_of" "rules" "preconditions"
    "postconditions" "slot_conditions" "array"
    "dimensions" "exact_cardinality")
  "LinkML keywords.")

(defconst linkml-builtin-types
  '("string" "integer" "float" "double" "boolean"
    "date" "datetime" "time" "uri" "uriorcurie"
    "curie" "ncname" "nodeidentifier" "jsonpointer"
    "jsonpath" "sparqlpath")
  "LinkML built-in types.")

(defconst linkml-font-lock-keywords
  `((,(regexp-opt linkml-keywords 'words) . font-lock-keyword-face)
    (,(regexp-opt linkml-builtin-types 'words) . font-lock-type-face)
    ("^\\([a-zA-Z_][a-zA-Z0-9_]*\\):" . (1 font-lock-function-name-face))
    ("\\(true\\|false\\|yes\\|no\\|null\\)" . font-lock-constant-face)
    ("\\(https?://[^ \n]+\\)" . font-lock-string-face))
  "Font lock keywords for LinkML mode.")

;;; Syntax table

(defvar linkml-mode-syntax-table
  (let ((table (make-syntax-table yaml-mode-syntax-table)))
    (modify-syntax-entry ?_ "w" table)
    (modify-syntax-entry ?- "w" table)
    table)
  "Syntax table for LinkML mode.")

;;; Imenu support

(defun linkml-imenu-create-index ()
  "Create an imenu index for LinkML schemas."
  (let ((index '())
        (class-index '())
        (slot-index '())
        (type-index '())
        (enum-index '()))
    (save-excursion
      (goto-char (point-min))
      ;; Find classes
      (when (re-search-forward "^classes:" nil t)
        (while (re-search-forward "^  \\([a-zA-Z_][a-zA-Z0-9_]*\\):" nil t)
          (push (cons (match-string 1) (match-beginning 0)) class-index)))
      ;; Find slots
      (goto-char (point-min))
      (when (re-search-forward "^slots:" nil t)
        (while (re-search-forward "^  \\([a-zA-Z_][a-zA-Z0-9_]*\\):" nil t)
          (push (cons (match-string 1) (match-beginning 0)) slot-index)))
      ;; Find types
      (goto-char (point-min))
      (when (re-search-forward "^types:" nil t)
        (while (re-search-forward "^  \\([a-zA-Z_][a-zA-Z0-9_]*\\):" nil t)
          (push (cons (match-string 1) (match-beginning 0)) type-index)))
      ;; Find enums
      (goto-char (point-min))
      (when (re-search-forward "^enums:" nil t)
        (while (re-search-forward "^  \\([a-zA-Z_][a-zA-Z0-9_]*\\):" nil t)
          (push (cons (match-string 1) (match-beginning 0)) enum-index))))
    ;; Build the index
    (when class-index
      (push (cons "Classes" (nreverse class-index)) index))
    (when slot-index
      (push (cons "Slots" (nreverse slot-index)) index))
    (when type-index
      (push (cons "Types" (nreverse type-index)) index))
    (when enum-index
      (push (cons "Enums" (nreverse enum-index)) index))
    (nreverse index)))

;;; Flycheck integration

(flycheck-define-checker linkml
  "A LinkML schema checker using the linkml CLI."
  :command ("linkml" "validate" "--format" "json" source)
  :error-parser flycheck-parse-json
  :modes linkml-mode
  :predicate (lambda () (executable-find linkml-executable)))

(add-to-list 'flycheck-checkers 'linkml)

;;; Company completion

(defun linkml-company-backend (command &optional arg &rest ignored)
  "Company backend for LinkML completion.
COMMAND is the company command, ARG is the prefix or candidate."
  (interactive (list 'interactive))
  (cl-case command
    (interactive (company-begin-backend 'linkml-company-backend))
    (prefix (and (eq major-mode 'linkml-mode)
                 (company-grab-symbol)))
    (candidates
     (let ((prefix arg))
       (cl-remove-if-not
        (lambda (c) (string-prefix-p prefix c))
        (append linkml-keywords linkml-builtin-types))))
    (sorted t)
    (duplicates nil)))

;;; LSP support

(defun linkml-setup-lsp ()
  "Setup LSP for LinkML."
  (add-to-list 'lsp-language-id-configuration '(linkml-mode . "linkml"))
  (lsp-register-client
   (make-lsp-client
    :new-connection (lsp-stdio-connection (lambda () linkml-executable))
    :activation-fn (lsp-activate-on "linkml")
    :server-id 'linkml-ls
    :priority 0)))

;;; Interactive commands

(defun linkml-validate ()
  "Validate the current LinkML schema."
  (interactive)
  (let ((file (buffer-file-name)))
    (if file
        (let ((output (shell-command-to-string
                       (format "%s validate %s" linkml-executable file))))
          (if (string-match "valid" output)
              (message "Schema is valid!")
            (message "Validation errors: %s" output)))
      (message "Buffer must be saved to validate"))))

(defun linkml-generate-code ()
  "Generate code from the current LinkML schema."
  (interactive)
  (let ((file (buffer-file-name))
        (target (completing-read "Target language: "
                                 '("python" "pydantic" "typescript" "javascript"
                                   "java" "go" "rust" "sql" "graphql"
                                   "jsonschema" "shacl" "owl")
                                 nil t linkml-default-generation-target)))
    (if file
        (let* ((output-file (concat (file-name-sans-extension file)
                                    "." (cdr (assoc target
                                                    '(("python" . "py")
                                                      ("pydantic" . "py")
                                                      ("typescript" . "ts")
                                                      ("javascript" . "js")
                                                      ("java" . "java")
                                                      ("go" . "go")
                                                      ("rust" . "rs")
                                                      ("sql" . "sql")
                                                      ("graphql" . "graphql")
                                                      ("jsonschema" . "json")
                                                      ("shacl" . "ttl")
                                                      ("owl" . "owl"))))))
               (command (format "%s generate -t %s -o %s %s"
                                linkml-executable target output-file file)))
          (shell-command command)
          (find-file output-file)
          (message "Generated %s code in %s" target output-file))
      (message "Buffer must be saved to generate code"))))

(defun linkml-format ()
  "Format the current LinkML schema."
  (interactive)
  (let ((file (buffer-file-name)))
    (if file
        (let ((formatted (shell-command-to-string
                          (format "%s format %s" linkml-executable file))))
          (erase-buffer)
          (insert formatted)
          (message "Schema formatted"))
      (message "Buffer must be saved to format"))))

(defun linkml-convert-format ()
  "Convert LinkML schema to another format."
  (interactive)
  (let ((file (buffer-file-name))
        (format (completing-read "Target format: "
                                 '("json" "jsonld" "rdf" "ttl")
                                 nil t)))
    (if file
        (let* ((output-file (concat (file-name-sans-extension file) "." format))
               (command (format "%s convert -f %s -o %s %s"
                                linkml-executable format output-file file)))
          (shell-command command)
          (find-file output-file)
          (message "Converted to %s format in %s" format output-file))
      (message "Buffer must be saved to convert"))))

(defun linkml-visualize ()
  "Visualize the current LinkML schema."
  (interactive)
  (let ((file (buffer-file-name)))
    (if file
        (let ((dot-file (concat (file-name-sans-extension file) ".dot"))
              (png-file (concat (file-name-sans-extension file) ".png")))
          (shell-command (format "%s generate -t graphviz -o %s %s"
                                 linkml-executable dot-file file))
          (shell-command (format "dot -Tpng %s -o %s" dot-file png-file))
          (if (file-exists-p png-file)
              (progn
                (find-file png-file)
                (message "Schema visualization created: %s" png-file))
            (message "Failed to create visualization")))
      (message "Buffer must be saved to visualize"))))

(defun linkml-insert-class ()
  "Insert a LinkML class template."
  (interactive)
  (let ((name (read-string "Class name: ")))
    (insert (format "%s:\n  description: %s\n  attributes:\n    id:\n      identifier: true\n      range: string\n    "
                    name name))))

(defun linkml-insert-attribute ()
  "Insert a LinkML attribute template."
  (interactive)
  (let ((name (read-string "Attribute name: "))
        (type (completing-read "Type: " linkml-builtin-types nil nil "string")))
    (insert (format "%s:\n      description: \n      range: %s\n    "
                    name type))))

;;; Menu

(easy-menu-define linkml-mode-menu linkml-mode-map
  "Menu for LinkML mode."
  '("LinkML"
    ["Validate Schema" linkml-validate t]
    ["Generate Code..." linkml-generate-code t]
    ["Format Schema" linkml-format t]
    ["Convert Format..." linkml-convert-format t]
    ["Visualize Schema" linkml-visualize t]
    "---"
    ["Insert Class" linkml-insert-class t]
    ["Insert Attribute" linkml-insert-attribute t]
    "---"
    ["Customize" (customize-group 'linkml) t]))

;;; Mode definition

(define-derived-mode linkml-mode yaml-mode "LinkML"
  "Major mode for editing LinkML schema files.

\\{linkml-mode-map}"
  :group 'linkml
  (setq-local font-lock-defaults '(linkml-font-lock-keywords))
  (setq-local indent-line-function 'yaml-indent-line)
  (setq-local comment-start "# ")
  (setq-local comment-end "")
  (setq-local imenu-create-index-function 'linkml-imenu-create-index)

  ;; Setup company
  (add-to-list 'company-backends 'linkml-company-backend)

  ;; Setup flycheck
  (when (bound-and-true-p flycheck-mode)
    (flycheck-select-checker 'linkml))

  ;; Setup LSP if available
  (when (featurep 'lsp-mode)
    (linkml-setup-lsp)
    (lsp))

  ;; Add save hook
  (when linkml-validate-on-save
    (add-hook 'after-save-hook 'linkml-validate nil t)))

;;; File associations

(add-to-list 'auto-mode-alist '("\\.linkml\\.ya?ml\\'" . linkml-mode))
(add-to-list 'auto-mode-alist '("\\.linkml\\'" . linkml-mode))

;;; Snippets

(with-eval-after-load 'yasnippet
  (let ((dir (expand-file-name "snippets" (file-name-directory load-file-name))))
    (when (file-directory-p dir)
      (add-to-list 'yas-snippet-dirs dir))))

(provide 'linkml-mode)

;;; linkml-mode.el ends here
