" LinkML syntax highlighting for Vim
" Version: 2.0.0

if exists("b:current_syntax")
  finish
endif

" Include YAML syntax as base
runtime! syntax/yaml.vim
unlet! b:current_syntax

" LinkML Keywords
syntax keyword linkmlKeyword id name title description version license
syntax keyword linkmlKeyword prefixes default_prefix imports
syntax keyword linkmlKeyword classes slots types enums subsets
syntax keyword linkmlKeyword is_a mixins abstract mixin attributes
syntax keyword linkmlKeyword range required identifier multivalued
syntax keyword linkmlKeyword pattern minimum_value maximum_value
syntax keyword linkmlKeyword permissible_values slot_usage aliases
syntax keyword linkmlKeyword exact_mappings close_mappings mappings
syntax keyword linkmlKeyword examples see_also deprecated comments
syntax keyword linkmlKeyword domain slot_uri key designates_type
syntax keyword linkmlKeyword equals_string equals_number
syntax keyword linkmlKeyword minimum_cardinality maximum_cardinality
syntax keyword linkmlKeyword exactly_one_of any_of all_of none_of
syntax keyword linkmlKeyword rules preconditions postconditions
syntax keyword linkmlKeyword slot_conditions array dimensions
syntax keyword linkmlKeyword exact_cardinality tree_root
syntax keyword linkmlKeyword generation_date source_file source_file_date
syntax keyword linkmlKeyword source_file_size metamodel_version
syntax keyword linkmlKeyword default_range default_curi_maps
syntax keyword linkmlKeyword narrow_mappings broad_mappings related_mappings
syntax keyword linkmlKeyword rank in_subset owner imported_from
syntax keyword linkmlKeyword readonly ifabsent list_elements_unique
syntax keyword linkmlKeyword list_elements_ordered shared symmetric
syntax keyword linkmlKeyword reflexive locally_reflexive irreflexive
syntax keyword linkmlKeyword asymmetric transitive inverse inverse_of
syntax keyword linkmlKeyword subproperty_of symmetric reflexive
syntax keyword linkmlKeyword defining_slots unique_keys classification_rules
syntax keyword linkmlKeyword apply_to values_from string_serialization

" Built-in Types
syntax keyword linkmlType string integer float double boolean
syntax keyword linkmlType date datetime time uri uriorcurie
syntax keyword linkmlType curie ncname nodeidentifier
syntax keyword linkmlType jsonpointer jsonpath sparqlpath
syntax keyword linkmlType objectidentifier

" Boolean values
syntax keyword linkmlBoolean true false yes no

" Special values
syntax keyword linkmlSpecial null

" Class names (capitalized words after specific keywords)
syntax match linkmlClass "^\s*\zs[A-Z][A-Za-z0-9_]*\ze:"
syntax match linkmlClassRef "\<is_a:\s*\zs[A-Z][A-Za-z0-9_]*"
syntax match linkmlClassRef "\<mixins:\s*\zs[A-Z][A-Za-z0-9_]*"
syntax match linkmlClassRef "\<range:\s*\zs[A-Z][A-Za-z0-9_]*"

" Attribute names (lowercase words with colons)
syntax match linkmlAttribute "^\s\{2,\}\zs[a-z_][a-z0-9_]*\ze:"

" Slot names
syntax match linkmlSlot "^\s\+\zs[a-z_][a-z0-9_]*\ze:"

" URLs
syntax match linkmlURL "https\?://[^ ]\+"

" CURIEs (Compact URIs)
syntax match linkmlCURIE "\<\w\+:\w\+"

" Patterns (regex)
syntax region linkmlPattern start=/pattern:\s*'/ end=/'/ contains=linkmlRegex
syntax region linkmlPattern start=/pattern:\s*"/ end=/"/ contains=linkmlRegex
syntax match linkmlRegex /\^.*\$/ contained

" Numbers
syntax match linkmlNumber "\<\d\+\>"
syntax match linkmlNumber "\<\d\+\.\d\+\>"
syntax match linkmlNumber "\<\d\+[eE][+-]\?\d\+\>"
syntax match linkmlNumber "\<\d\+\.\d\+[eE][+-]\?\d\+\>"

" Version numbers
syntax match linkmlVersion "\<\d\+\.\d\+\.\d\+\>"

" Comments
syntax match linkmlComment "#.*$" contains=linkmlTodo
syntax keyword linkmlTodo TODO FIXME XXX NOTE contained

" String regions
syntax region linkmlString start=/'/ end=/'/ skip=/\\'/
syntax region linkmlString start=/"/ end=/"/ skip=/\\"/
syntax region linkmlString start=/|/ end=/^[^ ]/ contains=linkmlStringSpecial
syntax match linkmlStringSpecial /^\s*/ contained

" Multiline text blocks
syntax region linkmlText start=/>\s*$/ end=/^[^ ]/ contains=linkmlTextSpecial
syntax region linkmlText start=/|\s*$/ end=/^[^ ]/ contains=linkmlTextSpecial
syntax match linkmlTextSpecial /^\s*/ contained

" Highlight groups
highlight default link linkmlKeyword Keyword
highlight default link linkmlType Type
highlight default link linkmlBoolean Boolean
highlight default link linkmlSpecial Special
highlight default link linkmlClass Structure
highlight default link linkmlClassRef Type
highlight default link linkmlAttribute Identifier
highlight default link linkmlSlot Function
highlight default link linkmlURL Underlined
highlight default link linkmlCURIE Constant
highlight default link linkmlPattern String
highlight default link linkmlRegex SpecialChar
highlight default link linkmlNumber Number
highlight default link linkmlVersion Number
highlight default link linkmlComment Comment
highlight default link linkmlTodo Todo
highlight default link linkmlString String
highlight default link linkmlText String
highlight default link linkmlStringSpecial SpecialChar
highlight default link linkmlTextSpecial SpecialChar

" Set current syntax
let b:current_syntax = "linkml"
