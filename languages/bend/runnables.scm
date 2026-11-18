; `def main` gets a run button.
((function_definition
  name: (identifier) @run
  (#eq? @run "main"))
  (#set! tag bend-main))

; A law gets a button that checks the file's proofs.
((law_definition
  name: (identifier) @run @law)
  (#set! tag bend-law))
