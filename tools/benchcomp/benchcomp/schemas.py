# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Schemas for different file formats. Contained in a separate file so that
# validation is optional.


import textwrap

import schema



class _Schema:

    # This is to work around a problem with `schema` where it does not dump a
    # sub-schema to JSON when it is a schema.Literal with a type (e.g.
    # schema.Literal(str, ...). All subclasses of _Schema should use `_str` in
    # place of `str`; this method will transform it to the correct type for
    # validation. The documentation generator (which uses the JSON dumper)
    # avoids doing this and renders the raw schema directly.

    def _replace_types(self, schema_dict):
        if not isinstance(schema_dict, (dict, schema.Literal)):
            return schema_dict

        if not isinstance(schema_dict, (dict, )):
            if schema_dict.schema == "_str":
                return schema.Literal(str, schema_dict.description)
            return schema_dict

        tmp = {}
        for k, v in schema_dict.items():
            if k == "_str":
                tmp[str] = self._replace_types(v)
            elif isinstance(k, (schema.Literal)):
                if k.schema == "_str":
                    new_lit = schema.Literal(str, k.description)
                    tmp[new_lit] = self._replace_types(v)
                else:
                    tmp[k] = self._replace_types(v)
            else:
                tmp[k] = self._replace_types(v)
        return tmp


    def __call__(self):
        s = self.get_raw_schema()
        name, description = s["name"], s["description"]
        ret = self._replace_types(s["schema"])
        return schema.Schema(ret, name=name, description=description)



class BenchcompYaml(_Schema):
    def get_raw_schema(self):
      return {
        "schema": {
          "run": {
            schema.Literal("suites", description=(
              "a collection of benchmarks in a directory")): {
              schema.Literal("_str", description="ID for the suite"): {
                schema.Literal("variants", description=(
                  "list of variant IDs to run for this suite")): [ str ],
              schema.Literal("parser", description=textwrap.dedent(
                "program used to read the results of a suite after the run.")):
                schema.Or({
                    schema.Literal("command", description=(
                      "path to an executable parser")): str
                }, {
                    schema.Literal("module", description=(
                      "name of a parser module in benchcomp.parsers")): str
                })
              },
            },
          },
          schema.Literal("variants", description=(
          "configurations under which benchcomp will run the suites")): {
            schema.Literal("_str", description="ID for the variant"): {
              "config": {
                schema.Literal("command_line", description=(
                  "command for running a suite using this variant")): str,
                schema.Literal("directory", description=(
                  "path where this variant runs")): str,
                schema.Optional(schema.Literal("env", description=(
                  "overrides for environment variables"))): {
                  schema.Literal("_str"): schema.Literal("_str"),
                },
              },
            },
          },
          schema.Literal("visualize", description=textwrap.dedent("""\
            visualizations to apply to the result, see
            <a href="#built-in-visualizations">built-in visualizations</a>
            below """)): [
            dict
        ]},
        "name": "benchcomp.yaml",
        "description": textwrap.dedent("""\
          A configuration file that controls how benchcomp runs and combines
          the results of benchmark suites.""")
      }
