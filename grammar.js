/**
 * @file Grammar for Structurizr DSL for describing c4 models
 * @author Rob Hand <146272+sinon@users.noreply.github.com>
 * @license MIT
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

export default grammar({
  name: "structurizr",

  extras: $ => [
    /\s/,
    $.comment,
  ],

  rules: {
    source_file: $ => repeat($._definition),

    comment: _ => token(choice(
      seq("//", /.*/),
      seq("#", /.*/),
    )),

    identifier: _ => /[A-Za-z_][A-Za-z0-9_.-]*/,

    string: _ => token(seq(
      '"',
      repeat(choice(
        /[^"\\\n]+/,
        /\\./,
      )),
      '"',
    )),

    _value: $ => choice(
      $.string,
      $.identifier,
    ),

    _definition: $ => choice(
      $.workspace,
      $.model,
      $.views,
    ),

    workspace: $ => choice(
      seq(
        "workspace",
        field("body", $.workspace_block),
      ),
      seq(
        "workspace",
        field("name", $._value),
        field("body", $.workspace_block),
      ),
      seq(
        "workspace",
        field("name", $._value),
        field("description", $._value),
        field("body", $.workspace_block),
      ),
      seq(
        "workspace",
        "extends",
        field("base", $._value),
        field("body", $.workspace_block),
      ),
    ),

    workspace_block: $ => seq(
      "{",
      repeat(choice(
        $.name_statement,
        $.description_statement,
        $.model,
        $.views,
      )),
      "}",
    ),

    model: $ => seq(
      "model",
      field("body", $.model_block),
    ),

    model_block: _ => seq("{", "}"),

    views: $ => seq(
      "views",
      field("body", $.views_block),
    ),

    views_block: _ => seq("{", "}"),

    name_statement: $ => seq(
      "name",
      field("value", $._value),
    ),

    description_statement: $ => seq(
      "description",
      field("value", $._value),
    ),
  },
});
