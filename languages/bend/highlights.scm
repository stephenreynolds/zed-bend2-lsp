; Bend 2 highlights. When several patterns match a node, the last one wins,
; so general rules come first and structural roles refine them below.
;
; Adapted from the queries shipped with Soulthym/tree-sitter-bend2 and
; IlyaGulya/bend2-zed (both MIT); see THIRD_PARTY_NOTICES.md.

; ---------------------------------------------------------------- names

(identifier) @variable

; A dotted name is a single token (`U32.show`, `M.square`). A lowercase last
; segment names a def; an uppercase one names a type.
((identifier) @function
  (#match? @function "^[A-Za-z_][A-Za-z0-9_]*(\\.[A-Za-z0-9_]+)*\\.[a-z_][A-Za-z0-9_]*$"))

((identifier) @type
  (#match? @type "^([A-Za-z_][A-Za-z0-9_]*\\.)*[A-Z][A-Za-z0-9_]*$"))

((identifier) @type.builtin
  (#any-of? @type.builtin
    "Empty" "Unit" "Bool" "Cmp" "Either" "Sigma" "Pair" "Nat" "U32" "F32"
    "Char" "String" "Maybe" "Result" "List" "Word" "Array" "Map" "Set"
    "Equal" "IO" "Chan" "File" "Socket" "Listener" "Window" "Audio" "Image"
    "Event" "App"))

((identifier) @variable.special
  (#eq? @variable.special "_"))

; ---------------------------------------------------------- definitions

(function_definition name: (identifier) @function)
(law_definition name: (identifier) @function)
(type_definition name: (identifier) @type)
(constructor_definition name: (identifier) @constructor)

; ---------------------------------------------------------------- uses

(constructor_expression name: (identifier) @constructor)
(eliminator_arm name: (identifier) @constructor)
(type_application name: (identifier) @type)
(do_expression monad: (identifier) @type)

; Calls of capitalized names stay types: `IO(Unit)`, `Chan(String)`, `IsEven(n)`.
((call_expression function: (identifier) @function)
  (#not-match? @function "^([A-Za-z_][A-Za-z0-9_]*\\.)*[A-Z][A-Za-z0-9_]*$"))

((constructor_expression name: (identifier) @boolean)
  (#any-of? @boolean "True" "False"))

; ------------------------------------------------------------- binders

(parameter name: (identifier) @variable.parameter)
(type_parameter name: (identifier) @variable.parameter)
(constructor_definition (type_parameter name: (identifier) @property))
(law_clause name: (identifier) @variable.parameter)
(dependent_type name: (identifier) @variable.parameter)
(lambda_expression parameter: (identifier) @variable.parameter)
(lambda_expression
  parameter: (reusable_expression (identifier) @variable.parameter))
(lambda_expression
  parameter: (parenthesized_expression
    body: (body (identifier) @variable.parameter)))
(lambda_expression
  parameter: (parenthesized_expression
    body: (body (reusable_expression (identifier) @variable.parameter))))

; ------------------------------------------------------------ builtins

(builtin_type) @type.builtin
"Kind" @type.builtin
(quantity) @constant.builtin

; ------------------------------------------------------------ literals

(string) @string
(character) @string
(escape_sequence) @string.escape
(integer) @number
(natural) @number
(float) @number
(comment) @comment

; ------------------------------------------------------------- imports

(import_statement path: (import_path) @string.special)
(import_statement alias: (module_alias) @variable @namespace)
(foreign_import path: (string) @string.special)
"Base" @variable @namespace

; ------------------------------------------------------------ keywords

[
  "def" "law" "type" "is" "where" "for" "exs"
  "do" "match" "case" "return" "import" "as"
] @keyword

; While a line is being typed the parser may hold a keyword inside an ERROR
; node as a plain identifier; keep it colored as a keyword.
((ERROR (identifier) @keyword)
  (#any-of? @keyword
    "def" "law" "type" "is" "where" "for" "exs"
    "do" "match" "case" "return" "import" "as"))

; ------------------------------------------- operators and punctuation

[
  "->" "=>" "<-" "=" "==" "!=" "@" "\\" "~"
  "+" "-" "*" "/" "%" "^"
  "<" "<=" ">" ">=" "<<" ">>"
  "&" "|" "&&" "||" "<>" "++" "<&>" ".&." ".|." ".^."
] @operator

["(" ")" "[" "]" "{" "}"] @punctuation.bracket
(type_application ["<" ">"] @punctuation.bracket)
(type_parameters ["<" ">"] @punctuation.bracket)
(do_expression ["<" ">"] @punctuation.bracket)
["," ":" ";"] @punctuation.delimiter

; --------------------------------------------------- markers and holes

(decorator ["@" "unsafe"] @attribute)
(function_definition "?" @attribute)
(gpu_call "!" @punctuation.special)
(hole "?" @punctuation.special name: (identifier) @label)
(rewrite_expression name: (identifier) @label)
