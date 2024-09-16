// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::io::{BufRead, IsTerminal};
use std::{fs::File, io::BufReader, path::PathBuf};

use anyhow::Result;

use crate::coverage::{
    function_coverage_results, function_info_from_file, CovResult, FileCoverageInfo, FunctionInfo,
};
use crate::summary::{line_coverage_info, line_coverage_results};
use crate::{args::ReportArgs, coverage::CombinedCoverageResults};
// use crate::coverage::CoverageResults;
// use args::Args;

pub fn report_main(args: &ReportArgs) -> Result<()> {
    let mapfile = File::open(&args.mapfile)?;
    let reader = BufReader::new(mapfile);

    let covfile = File::open(&args.profile)?;
    let covreader = BufReader::new(covfile);
    let results: CombinedCoverageResults =
        serde_json::from_reader(covreader).expect("could not load coverage results");

    let source_files: Vec<PathBuf> =
        serde_json::from_reader(reader).expect("could not parse coverage metadata");

    for file in source_files {
        let fun_info = function_info_from_file(&file);
        let mut file_cov_info = Vec::new();
        for info in fun_info {
            let cov_results = function_coverage_results(&info, &file, &results);
            let line_coverage = line_coverage_results(&info, &cov_results);
            let line_coverage_matched: Vec<(usize, Option<(u32, Vec<CovResult>)>)> =
                (info.start.0..=info.end.0).zip(line_coverage.clone()).collect();
            // println!("REG: {line_coverage:?}");
            // println!("MATCHED: {line_coverage_matched:?}");
            // let new_res = line_coverage_matched.into_iter().filter(|(num, data)| data.is_some()).collect();
            file_cov_info.push(line_coverage_matched);
        }
        // let file_coverage_results = match_line_coverage_file(file_cov_info);
        print_coverage_results(file, file_cov_info)?;
    }

    Ok(())
}

pub fn validate_report_args(_args: &ReportArgs) -> Result<()> {
    Ok(())
}

pub fn print_coverage_results(
    filepath: PathBuf,
    results: Vec<Vec<(usize, Option<(u32, Vec<CovResult>)>)>>,
) -> Result<()> {
    let flattened_results: Vec<(usize, Option<(u32, Vec<CovResult>)>)> =
        results.into_iter().flatten().collect();
    println!("{}", filepath.to_string_lossy().to_string());

    let file = File::open(filepath)?;
    let reader = BufReader::new(file);

    println!("{flattened_results:?}");
    for (i, line) in reader.lines().enumerate() {
        let idx = i + 1;
        let line = line?;
        let cur_line_result = flattened_results.iter().find(|(num, _)| *num == idx);
        let max = if let Some((num, data)) = cur_line_result {
            if data.is_some() {
                format!("{:4}", data.clone().unwrap().0)
            } else {
                format!("{:4}", " ".to_string())
            }
        } else {
            format!("{:4}", " ".to_string())
        };
        let line_fmt = if max == "   0" { format!("{}{line}{}", "\x1b[42m", "\x1b[0m") } else {line};
        println!("{idx:4}| {max}| {line_fmt}");
        let differing_results: Vec<CovResult> =
            if let Some((num, data)) = cur_line_result { if data.is_some() {data.clone().unwrap().1} else {vec![]} } else { vec![] };
        let zero_differing_results: Vec<&CovResult> =
            differing_results.iter().filter(|x| x.times_covered == 0).collect();
        let mut str = std::iter::repeat(' ').take(11_usize).collect::<String>();
        let mut cur_shift = 0;
        let mut print_differing = false;
        for res in zero_differing_results {
            let start: usize = res.region.start.1.try_into().unwrap();
            let spaces_next = std::iter::repeat(' ').take(start - cur_shift).collect::<String>();
            str.push_str(&format!("{spaces_next}^0"));
            cur_shift += start + 2;
            print_differing = true;
            // res.region.start
        }
        if print_differing {
            println!("{str}");
        }
    }

    Ok(())
}
// fn match_line_coverage_file(line_cov_info: Vec<(FunctionInfo, Vec<Option<(u32, Vec<crate::coverage::CovResult>)>) -> ?? {
// }
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

fn red() -> String {
    let support_color = std::io::stdout().is_terminal();
    if support_color {
        "\x1b[42m".to_string()
    } else {
        "```".to_string()
    }
}
