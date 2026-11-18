(decorator) @annotation

(function_definition
  "def" @context
  name: (identifier) @name) @item

(law_definition
  "law" @context
  name: (identifier) @name) @item

(type_definition
  "type" @context
  name: (identifier) @name) @item

(constructor_definition
  name: (identifier) @name) @item
