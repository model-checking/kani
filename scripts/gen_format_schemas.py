#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

import inspect
import json
import pathlib
import sys
import tempfile

import json_schema_for_humans as jsfh
import json_schema_for_humans.generation_configuration
import json_schema_for_humans.generate
import schema

sys.path.append(str(pathlib.Path(__file__).parent.parent / "tools" / "benchcomp"))

# autopep8: off
import benchcomp.schemas
# autopep8: on


def main():
    out_dir = pathlib.Path(sys.argv[1]).resolve()
    out_dir.mkdir(exist_ok=True, parents=True)

    template = (
        pathlib.Path(__file__).parent.parent /
        "docs/src/schema.jinja2.html").resolve()
    config = jsfh.generation_configuration.GenerationConfiguration(
        template_name="md", examples_as_yaml=True, with_footer=False,
        copy_css=False, copy_js=False, custom_template_path=template)

    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = pathlib.Path(tmpdir)
        for name, klass in inspect.getmembers(benchcomp.schemas):
            if not inspect.isclass(klass):
                continue
            if name.startswith("_"):
                continue

            s = schema.Schema(klass().get_raw_schema()["schema"])
            schema_docs = s.json_schema(
                f"https://github.com/model-checking/kani/benchcomp/{name}")

            with open(tmpdir / f"{name}.json", "w") as handle:
                print(json.dumps(schema_docs, indent=2), file=handle)
            with open(f"/tmp/{name}.json", "w") as handle:
                print(json.dumps(schema_docs, indent=2), file=handle)

            jsfh.generate.generate_from_filename(
                tmpdir, out_dir / f"{name}.md", config=config)


if __name__ == "__main__":
    main()
