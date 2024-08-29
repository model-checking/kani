// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;

use crate::args::ReportArgs;
// use coverage::CoverageCheck;
// use crate::coverage::CoverageResults;
// use args::Args;

pub fn report_main(_args: &ReportArgs) -> Result<()> {
    Ok(())
}

pub fn validate_report_args(_args: &ReportArgs) -> Result<()> {
    Ok(())
}

// fn visualize_coverage_results(cov_results: &CoverageResults, root_path: PathBuf) -> Result<String> {
//     let mut formatted_output = String::new();
//     let cov_data = &cov_results.data;

//     for file in cov_data.keys() {
//         let file_path = root_path.join(file);
//         let file_handle = File::open(file_path)?;
//         let reader = BufReader::new(file_handle);

//         let checks = cov_data.get(file).unwrap().to_vec();
//         let mut must_highlight = false;

//         for (idx, line) in reader.lines().enumerate() {
//             let line = format!("{}\n", line.unwrap());

//             let cur_idx = idx + 1;
//             let line_checks: Vec<&CoverageCheck> = checks
//                 .iter()
//                 .filter(|c| {
//                     c.is_covered()
//                         && (cur_idx == c.region.start.0 as usize
//                             || cur_idx == c.region.end.0 as usize)
//                 })
//                 .collect();
//             let new_line = if line_checks.is_empty() {
//                 if must_highlight {
//                     insert_escapes(&line, vec![(0, true), (line.len() - 1, false)])
//                 } else {
//                     line
//                 }
//             } else {
//                 let mut markers = vec![];
//                 if must_highlight {
//                     markers.push((0, true))
//                 };

//                 for check in line_checks {
//                     let start_line = check.region.start.0 as usize;
//                     let start_column = (check.region.start.1 - 1u32) as usize;
//                     let end_line = check.region.end.0 as usize;
//                     let end_column = (check.region.end.1 - 1u32) as usize;
//                     if start_line == cur_idx {
//                         markers.push((start_column, true))
//                     }
//                     if end_line == cur_idx {
//                         markers.push((end_column, false))
//                     }
//                 }

//                 if markers.last().unwrap().1 {
//                     must_highlight = true;
//                     markers.push((line.len() - 1, false))
//                 } else {
//                     must_highlight = false;
//                 }
//                 println!("{:?}", markers);
//                 insert_escapes(&line, markers)
//             };
//             formatted_output.push_str(&new_line);
//         }
//     }
//     Ok(formatted_output)
// }

// fn cargo_root_dir(filepath: PathBuf) -> Option<PathBuf> {
//     let mut cargo_root_path = filepath.clone();
//     while !cargo_root_path.join("Cargo.toml").exists() {
//         let pop_result = cargo_root_path.pop();
//         if !pop_result {
//             return None;
//         }
//     }
//     Some(cargo_root_path)
// }

// fn insert_escapes(str: &String, markers: Vec<(usize, bool)>) -> String {
//     let mut new_str = str.clone();
//     let mut offset = 0;

//     let sym_markers = markers.iter().map(|(i, b)| (i, if *b { "\x1b[42m" } else { "\x1b[0m" }));
//     // let sym_markers = markers.iter().map(|(i, b)| (i, if *b { "```" } else { "'''" }));
//     for (i, b) in sym_markers {
//         println!("{}", i + offset);
//         new_str.insert_str(i + offset, b);
//         offset = offset + b.bytes().len();
//     }
//     new_str
// }

// fn open_marker() -> String {
//     let support_color = std::io::stdout().is_terminal();
//     if support_color {
//         "\x1b[42m".to_string()
//     } else {
//         "```".to_string()
//     }
// }
