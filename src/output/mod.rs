// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::cli::Algorithm;
use crate::facts::{AllFacts, Loan, Point, Region};
use crate::intern::InternerTables;
use fxhash::FxHashMap;
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};
use std::path::PathBuf;

mod datafrog_opt;
mod dump;
mod location_insensitive;
mod naive;
mod tracking;

#[derive(Clone, Debug)]
crate struct Output {
    crate borrow_live_at: FxHashMap<Point, Vec<Loan>>,

    dump_enabled: bool,

    // these are just for debugging
    restricts: FxHashMap<Point, BTreeMap<Region, BTreeSet<Loan>>>,
    restricts_anywhere: FxHashMap<Region, BTreeSet<Loan>>,
    region_live_at: FxHashMap<Point, Vec<Region>>,
    invalidates: FxHashMap<Point, Vec<Loan>>,
    crate potential_errors: FxHashMap<Point, Vec<Loan>>,
    subset: FxHashMap<Point, BTreeMap<Region, BTreeSet<Region>>>,
    subset_anywhere: FxHashMap<Region, BTreeSet<Region>>,
    crate region_degrees: tracking::RegionDegrees,
}

impl Output {
    crate fn compute(all_facts: &AllFacts, algorithm: Algorithm, dump_enabled: bool) -> Self {
        match algorithm {
            Algorithm::Naive => naive::compute(dump_enabled, all_facts.clone()),
            Algorithm::DatafrogOpt => datafrog_opt::compute(dump_enabled, all_facts.clone()),
            Algorithm::LocationInsensitive => {
                location_insensitive::compute(dump_enabled, all_facts.clone())
            }
        }
    }

    fn new(dump_enabled: bool) -> Self {
        Output {
            borrow_live_at: FxHashMap::default(),
            restricts: FxHashMap::default(),
            restricts_anywhere: FxHashMap::default(),
            region_live_at: FxHashMap::default(),
            invalidates: FxHashMap::default(),
            potential_errors: FxHashMap::default(),
            subset: FxHashMap::default(),
            subset_anywhere: FxHashMap::default(),
            region_degrees: tracking::RegionDegrees::new(),
            dump_enabled,
        }
    }

    crate fn dump(&self, output_dir: &Option<PathBuf>, intern: &InternerTables) -> io::Result<()> {
        dump::dump_rows(
            &mut writer_for(output_dir, "borrow_live_at")?,
            intern,
            &self.borrow_live_at,
        )?;

        if self.dump_enabled {
            dump::dump_rows(
                &mut writer_for(output_dir, "restricts")?,
                intern,
                &self.restricts,
            )?;
            dump::dump_rows(
                &mut writer_for(output_dir, "restricts_anywhere")?,
                intern,
                &self.restricts_anywhere,
            )?;
            dump::dump_rows(
                &mut writer_for(output_dir, "region_live_at")?,
                intern,
                &self.region_live_at,
            )?;
            dump::dump_rows(
                &mut writer_for(output_dir, "invalidates")?,
                intern,
                &self.invalidates,
            )?;
            dump::dump_rows(
                &mut writer_for(output_dir, "potential_errors")?,
                intern,
                &self.potential_errors,
            )?;
            dump::dump_rows(&mut writer_for(output_dir, "subset")?, intern, &self.subset)?;
            dump::dump_rows(
                &mut writer_for(output_dir, "subset_anywhere")?,
                intern,
                &self.subset_anywhere,
            )?;
        }
        return Ok(());

        fn writer_for(out_dir: &Option<PathBuf>, name: &str) -> io::Result<Box<Write>> {
            // create a writer for the provided output.
            // If we have an output directory use that, otherwise just dump to stdout
            use std::fs;

            Ok(match out_dir {
                Some(dir) => {
                    fs::create_dir_all(&dir)?;
                    let mut of = dir.join(name);
                    of.set_extension("facts");
                    Box::new(fs::File::create(of)?)
                }
                None => {
                    let mut stdout = io::stdout();
                    write!(&mut stdout, "# {}\n\n", name)?;
                    Box::new(stdout)
                }
            })
        }
    }

    crate fn borrows_in_scope_at(&self, location: Point) -> &[Loan] {
        match self.borrow_live_at.get(&location) {
            Some(p) => p,
            None => &[],
        }
    }

    crate fn restricts_at(&self, location: Point) -> Cow<'_, BTreeMap<Region, BTreeSet<Loan>>> {
        assert!(self.dump_enabled);
        match self.restricts.get(&location) {
            Some(map) => Cow::Borrowed(map),
            None => Cow::Owned(BTreeMap::default()),
        }
    }

    crate fn regions_live_at(&self, location: Point) -> &[Region] {
        assert!(self.dump_enabled);
        match self.region_live_at.get(&location) {
            Some(v) => v,
            None => &[],
        }
    }

    crate fn subsets_at(&self, location: Point) -> Cow<'_, BTreeMap<Region, BTreeSet<Region>>> {
        assert!(self.dump_enabled);
        match self.subset.get(&location) {
            Some(v) => Cow::Borrowed(v),
            None => Cow::Owned(BTreeMap::default()),
        }
    }
}
