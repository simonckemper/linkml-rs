" LinkML support for Vim
" Version: 2.0.0
" Author: RootReal Team
" License: MIT

if exists('g:loaded_linkml')
  finish
endif
let g:loaded_linkml = 1

" Configuration variables
let g:linkml_executable = get(g:, 'linkml_executable', 'linkml')
let g:linkml_validate_on_save = get(g:, 'linkml_validate_on_save', 1)
let g:linkml_format_on_save = get(g:, 'linkml_format_on_save', 0)
let g:linkml_default_target = get(g:, 'linkml_default_target', 'python')
let g:linkml_fold_enable = get(g:, 'linkml_fold_enable', 1)

" Commands
command! LinkMLValidate call linkml#Validate()
command! LinkMLGenerate call linkml#GenerateCode()
command! LinkMLFormat call linkml#Format()
command! LinkMLConvert call linkml#Convert()
command! LinkMLVisualize call linkml#Visualize()
command! LinkMLShowDocs call linkml#ShowDocumentation()
command! -nargs=? LinkMLNew call linkml#NewSchema(<q-args>)

" Mappings
nnoremap <silent> <leader>lv :LinkMLValidate<CR>
nnoremap <silent> <leader>lg :LinkMLGenerate<CR>
nnoremap <silent> <leader>lf :LinkMLFormat<CR>
nnoremap <silent> <leader>lc :LinkMLConvert<CR>
nnoremap <silent> <leader>lz :LinkMLVisualize<CR>
nnoremap <silent> <leader>ld :LinkMLShowDocs<CR>

" Auto commands
augroup linkml
  autocmd!
  autocmd BufNewFile,BufRead *.linkml.yaml,*.linkml.yml,*.linkml setfiletype linkml
  autocmd FileType linkml call linkml#Setup()

  if g:linkml_validate_on_save
    autocmd BufWritePost *.linkml.yaml,*.linkml.yml,*.linkml call linkml#Validate()
  endif

  if g:linkml_format_on_save
    autocmd BufWritePre *.linkml.yaml,*.linkml.yml,*.linkml call linkml#Format()
  endif
augroup END

" Menu
if has('menu')
  amenu 50.10 &LinkML.&Validate\ Schema<Tab>\\lv :LinkMLValidate<CR>
  amenu 50.20 &LinkML.&Generate\ Code<Tab>\\lg :LinkMLGenerate<CR>
  amenu 50.30 &LinkML.&Format\ Schema<Tab>\\lf :LinkMLFormat<CR>
  amenu 50.40 &LinkML.&Convert\ Format<Tab>\\lc :LinkMLConvert<CR>
  amenu 50.50 &LinkML.&Visualize\ Schema<Tab>\\lz :LinkMLVisualize<CR>
  amenu 50.60 &LinkML.-sep1- :
  amenu 50.70 &LinkML.&Documentation<Tab>\\ld :LinkMLShowDocs<CR>
  amenu 50.80 &LinkML.&New\ Schema :LinkMLNew<CR>
endif
