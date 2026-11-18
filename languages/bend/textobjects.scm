(comment)+ @comment.around

(function_definition
  body: (_) @function.inside) @function.around

(law_definition
  body: (_) @function.inside) @function.around

(type_definition
  (constructor_definition)* @class.inside) @class.around
