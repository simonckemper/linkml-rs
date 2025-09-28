" LinkML autoload functions
" Version: 2.0.0

" Setup function called when LinkML file is opened
function! linkml#Setup() abort
  " Set local options
  setlocal expandtab
  setlocal shiftwidth=2
  setlocal softtabstop=2
  setlocal tabstop=2
  setlocal textwidth=0
  setlocal commentstring=#\ %s

  " Enable folding if configured
  if g:linkml_fold_enable
    setlocal foldmethod=expr
    setlocal foldexpr=linkml#FoldExpr(v:lnum)
    setlocal foldlevel=1
  endif

  " Set up completion
  setlocal omnifunc=linkml#Complete

  " Set up abbreviations
  call linkml#SetupAbbreviations()
endfunction

" Validate the current schema
function! linkml#Validate() abort
  if !executable(g:linkml_executable)
    echohl ErrorMsg | echo "LinkML executable not found: " . g:linkml_executable | echohl None
    return
  endif

  let l:file = expand('%:p')
  if empty(l:file)
    echohl ErrorMsg | echo "Buffer must be saved to validate" | echohl None
    return
  endif

  echo "Validating LinkML schema..."
  let l:cmd = g:linkml_executable . ' validate ' . shellescape(l:file)
  let l:output = system(l:cmd)

  if v:shell_error == 0
    echohl MoreMsg | echo "Schema is valid!" | echohl None

    " Clear any existing error signs
    if exists('*sign_unplace')
      call sign_unplace('LinkMLErrors', {'buffer': bufnr('%')})
    endif
  else
    echohl ErrorMsg | echo "Validation errors found" | echohl None

    " Parse and display errors
    call linkml#ShowErrors(l:output)
  endif
endfunction

" Generate code from the schema
function! linkml#GenerateCode() abort
  if !executable(g:linkml_executable)
    echohl ErrorMsg | echo "LinkML executable not found" | echohl None
    return
  endif

  let l:file = expand('%:p')
  if empty(l:file)
    echohl ErrorMsg | echo "Buffer must be saved to generate code" | echohl None
    return
  endif

  " Get target language
  let l:targets = ['python', 'pydantic', 'typescript', 'javascript', 'java', 'go', 'rust', 'sql', 'graphql', 'jsonschema']
  let l:target = inputlist(['Select target language:'] + map(copy(l:targets), 'v:key + 1 . ". " . v:val'))
  if l:target < 1 || l:target > len(l:targets)
    return
  endif
  let l:target = l:targets[l:target - 1]

  " Determine output file extension
  let l:extensions = {
    \ 'python': 'py',
    \ 'pydantic': 'py',
    \ 'typescript': 'ts',
    \ 'javascript': 'js',
    \ 'java': 'java',
    \ 'go': 'go',
    \ 'rust': 'rs',
    \ 'sql': 'sql',
    \ 'graphql': 'graphql',
    \ 'jsonschema': 'json'
    \ }

  let l:output = fnamemodify(l:file, ':r') . '.' . l:extensions[l:target]

  echo "Generating " . l:target . " code..."
  let l:cmd = g:linkml_executable . ' generate -t ' . l:target . ' -o ' . shellescape(l:output) . ' ' . shellescape(l:file)
  let l:result = system(l:cmd)

  if v:shell_error == 0
    echohl MoreMsg | echo "Generated " . l:target . " code in " . l:output | echohl None

    " Ask if user wants to open the generated file
    if confirm("Open generated file?", "&Yes\n&No", 1) == 1
      execute 'edit ' . l:output
    endif
  else
    echohl ErrorMsg | echo "Generation failed: " . l:result | echohl None
  endif
endfunction

" Format the current schema
function! linkml#Format() abort
  if !executable(g:linkml_executable)
    echohl ErrorMsg | echo "LinkML executable not found" | echohl None
    return
  endif

  let l:file = expand('%:p')
  if empty(l:file)
    echohl ErrorMsg | echo "Buffer must be saved to format" | echohl None
    return
  endif

  " Save cursor position
  let l:save_cursor = getpos('.')

  " Format the file
  let l:cmd = g:linkml_executable . ' format ' . shellescape(l:file)
  let l:formatted = system(l:cmd)

  if v:shell_error == 0
    " Replace buffer contents
    let l:save_view = winsaveview()
    silent %delete _
    put =l:formatted
    silent 1delete _
    call winrestview(l:save_view)

    echo "Schema formatted"
  else
    echohl ErrorMsg | echo "Format failed: " . l:formatted | echohl None
  endif

  " Restore cursor position
  call setpos('.', l:save_cursor)
endfunction

" Convert schema to another format
function! linkml#Convert() abort
  if !executable(g:linkml_executable)
    echohl ErrorMsg | echo "LinkML executable not found" | echohl None
    return
  endif

  let l:file = expand('%:p')
  if empty(l:file)
    echohl ErrorMsg | echo "Buffer must be saved to convert" | echohl None
    return
  endif

  let l:formats = ['json', 'jsonld', 'rdf', 'ttl']
  let l:format = inputlist(['Select target format:'] + map(copy(l:formats), 'v:key + 1 . ". " . v:val'))
  if l:format < 1 || l:format > len(l:formats)
    return
  endif
  let l:format = l:formats[l:format - 1]

  let l:output = fnamemodify(l:file, ':r') . '.' . l:format

  echo "Converting to " . l:format . " format..."
  let l:cmd = g:linkml_executable . ' convert -f ' . l:format . ' -o ' . shellescape(l:output) . ' ' . shellescape(l:file)
  let l:result = system(l:cmd)

  if v:shell_error == 0
    echohl MoreMsg | echo "Converted to " . l:format . " format in " . l:output | echohl None

    if confirm("Open converted file?", "&Yes\n&No", 1) == 1
      execute 'edit ' . l:output
    endif
  else
    echohl ErrorMsg | echo "Conversion failed: " . l:result | echohl None
  endif
endfunction

" Visualize the schema
function! linkml#Visualize() abort
  if !executable(g:linkml_executable)
    echohl ErrorMsg | echo "LinkML executable not found" | echohl None
    return
  endif

  if !executable('dot')
    echohl ErrorMsg | echo "Graphviz 'dot' command not found" | echohl None
    return
  endif

  let l:file = expand('%:p')
  if empty(l:file)
    echohl ErrorMsg | echo "Buffer must be saved to visualize" | echohl None
    return
  endif

  let l:dot = fnamemodify(l:file, ':r') . '.dot'
  let l:png = fnamemodify(l:file, ':r') . '.png'

  echo "Generating visualization..."
  let l:cmd = g:linkml_executable . ' generate -t graphviz -o ' . shellescape(l:dot) . ' ' . shellescape(l:file)
  let l:result = system(l:cmd)

  if v:shell_error == 0
    " Convert dot to PNG
    let l:cmd = 'dot -Tpng ' . shellescape(l:dot) . ' -o ' . shellescape(l:png)
    let l:result = system(l:cmd)

    if v:shell_error == 0
      echohl MoreMsg | echo "Visualization created: " . l:png | echohl None

      " Open the image
      if has('mac')
        call system('open ' . shellescape(l:png))
      elseif has('unix')
        call system('xdg-open ' . shellescape(l:png))
      elseif has('win32')
        call system('start ' . shellescape(l:png))
      endif
    else
      echohl ErrorMsg | echo "Failed to create PNG: " . l:result | echohl None
    endif
  else
    echohl ErrorMsg | echo "Failed to generate dot file: " . l:result | echohl None
  endif
endfunction

" Show documentation for word under cursor
function! linkml#ShowDocumentation() abort
  let l:word = expand('<cword>')

  " LinkML keyword documentation
  let l:docs = {
    \ 'classes': 'Defines the classes (entities) in the schema',
    \ 'attributes': 'Defines attributes (properties) of a class',
    \ 'slots': 'Defines reusable slots that can be referenced by classes',
    \ 'types': 'Defines custom types based on built-in types',
    \ 'enums': 'Defines enumerations with permissible values',
    \ 'range': 'Specifies the type or class that this attribute can hold',
    \ 'required': 'Whether this attribute must be provided',
    \ 'identifier': 'Whether this attribute serves as the unique identifier',
    \ 'multivalued': 'Whether this attribute can have multiple values',
    \ 'pattern': 'Regular expression pattern for string validation',
    \ 'is_a': 'Parent class for inheritance',
    \ 'mixins': 'Classes to mix in (multiple inheritance)',
    \ 'abstract': 'Whether this class is abstract (cannot be instantiated)',
    \ 'minimum_value': 'Minimum allowed value for numeric types',
    \ 'maximum_value': 'Maximum allowed value for numeric types',
    \ 'permissible_values': 'List of allowed values for enums'
    \ }

  if has_key(l:docs, l:word)
    echo l:word . ': ' . l:docs[l:word]
  else
    echo "No documentation found for '" . l:word . "'"
  endif
endfunction

" Create a new schema
function! linkml#NewSchema(name) abort
  let l:name = empty(a:name) ? input('Schema name: ') : a:name
  if empty(l:name)
    return
  endif

  let l:template = [
    \ 'id: https://example.com/' . tolower(l:name),
    \ 'name: ' . l:name,
    \ 'description: ' . l:name . ' schema definition',
    \ 'version: 0.1.0',
    \ '',
    \ 'prefixes:',
    \ '  linkml: https://w3id.org/linkml/',
    \ '  ' . tolower(l:name) . ': https://example.com/' . tolower(l:name) . '/',
    \ '',
    \ 'default_prefix: ' . tolower(l:name),
    \ '',
    \ 'imports:',
    \ '  - linkml:types',
    \ '',
    \ 'classes:',
    \ '  ' . l:name . ':',
    \ '    description: Main ' . l:name . ' class',
    \ '    attributes:',
    \ '      id:',
    \ '        identifier: true',
    \ '        range: string',
    \ '        description: Unique identifier',
    \ '      name:',
    \ '        range: string',
    \ '        required: true',
    \ '        description: Name of the ' . tolower(l:name),
    \ '      description:',
    \ '        range: string',
    \ '        description: Optional description'
    \ ]

  " Create new buffer with template
  enew
  call setline(1, l:template)
  setfiletype linkml

  " Save if user provides filename
  let l:filename = input('Save as (empty to skip): ', l:name . '.linkml.yaml', 'file')
  if !empty(l:filename)
    execute 'write ' . l:filename
  endif
endfunction

" Omni completion function
function! linkml#Complete(findstart, base) abort
  if a:findstart
    " Find start of word
    let l:line = getline('.')
    let l:start = col('.') - 1
    while l:start > 0 && l:line[l:start - 1] =~ '\w'
      let l:start -= 1
    endwhile
    return l:start
  else
    " Return completions
    let l:keywords = [
      \ 'id', 'name', 'title', 'description', 'version', 'license',
      \ 'prefixes', 'default_prefix', 'imports',
      \ 'classes', 'slots', 'types', 'enums', 'subsets',
      \ 'is_a', 'mixins', 'abstract', 'mixin', 'attributes',
      \ 'range', 'required', 'identifier', 'multivalued',
      \ 'pattern', 'minimum_value', 'maximum_value',
      \ 'permissible_values', 'slot_usage', 'aliases',
      \ 'exact_mappings', 'close_mappings', 'mappings'
      \ ]

    let l:types = [
      \ 'string', 'integer', 'float', 'double', 'boolean',
      \ 'date', 'datetime', 'time', 'uri', 'uriorcurie',
      \ 'curie', 'ncname'
      \ ]

    let l:matches = []

    " Add keywords
    for l:keyword in l:keywords
      if l:keyword =~ '^' . a:base
        call add(l:matches, {'word': l:keyword, 'menu': 'keyword'})
      endif
    endfor

    " Add types
    for l:type in l:types
      if l:type =~ '^' . a:base
        call add(l:matches, {'word': l:type, 'menu': 'type'})
      endif
    endfor

    return l:matches
  endif
endfunction

" Folding expression
function! linkml#FoldExpr(lnum) abort
  let l:line = getline(a:lnum)
  let l:next = getline(a:lnum + 1)

  " Top level sections
  if l:line =~ '^\(classes\|slots\|types\|enums\|subsets\):'
    return '>1'
  endif

  " Second level items
  if l:line =~ '^  \w\+:' && l:next =~ '^    '
    return '>2'
  endif

  " Attributes section
  if l:line =~ '^    attributes:' && l:next =~ '^      '
    return '>3'
  endif

  " Individual attributes
  if l:line =~ '^      \w\+:' && l:next =~ '^        '
    return '>4'
  endif

  return '='
endfunction

" Set up abbreviations
function! linkml#SetupAbbreviations() abort
  " Common patterns
  iabbrev <buffer> cls classes:<CR>
  iabbrev <buffer> attr attributes:<CR>
  iabbrev <buffer> req required: true
  iabbrev <buffer> multi multivalued: true
  iabbrev <buffer> ident identifier: true
  iabbrev <buffer> desc description:
  iabbrev <buffer> rng range:
endfunction

" Show validation errors
function! linkml#ShowErrors(output) abort
  " Clear previous quickfix list
  call setqflist([])

  " Parse error output and add to quickfix
  let l:errors = []
  for l:line in split(a:output, '\n')
    if l:line =~ 'line \d\+'
      let l:matches = matchlist(l:line, 'line \(\d\+\)')
      if len(l:matches) > 1
        call add(l:errors, {
          \ 'filename': expand('%:p'),
          \ 'lnum': l:matches[1],
          \ 'text': l:line,
          \ 'type': 'E'
          \ })
      endif
    endif
  endfor

  if !empty(l:errors)
    call setqflist(l:errors)
    copen
  endif
endfunction
