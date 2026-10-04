use std::collections::BTreeMap;

use anyhow::{Result, ensure};
use expected_write::WritePlan;
use v30::AssembledProgram;

use super::super::interface_text::PlannedInterfaceTextWrites;
use super::super::mad_system_text::PlannedMadSystemTextWrites;
use super::super::payload_writes::PayloadFileWritePlan;
use super::super::shared_text::{CompiledGaijiBank, gaiji_bank_plan};

const MAD_FILE: &str = "MAD.COM";

pub(in crate::korean_patch) struct PlannedMadSystemInterfaceTextWrites {
    pub plans: Vec<PayloadFileWritePlan>,
    pub typed_sources: BTreeMap<String, AssembledProgram>,
}

pub(in crate::korean_patch) fn combine_mad_system_interface_text_plans(
    bank: &CompiledGaijiBank,
    system: PlannedMadSystemTextWrites,
    interface: PlannedInterfaceTextWrites,
) -> Result<PlannedMadSystemInterfaceTextWrites> {
    let system_plan = only_mad_plan(system.plans, "MAD system")?;
    let interface_plan = only_mad_plan(interface.plans, "MAD interface")?;
    ensure!(
        system_plan.resize.is_none() && interface_plan.resize.is_none(),
        "MAD text plans unexpectedly resize MAD.COM"
    );

    let mut typed_sources = system.typed_sources;
    for (id, source) in interface.typed_sources {
        ensure!(
            typed_sources.insert(id.clone(), source).is_none(),
            "MAD shared text duplicates typed source {id}"
        );
    }

    let mut regions = system_plan.regions;
    regions.extend(interface_plan.regions);
    let mut writes = system_plan.writes;
    writes.extend(interface_plan.writes);
    Ok(PlannedMadSystemInterfaceTextWrites {
        plans: vec![
            gaiji_bank_plan(
                bank,
                "mad-system-interface-text",
                "mad-system-interface-shared-gaiji-bank",
                "install one shared bank for MAD system and interface text",
            ),
            PayloadFileWritePlan {
                file_name: MAD_FILE,
                plan: WritePlan {
                    resize: None,
                    regions,
                    writes,
                },
            },
        ],
        typed_sources,
    })
}

fn only_mad_plan(plans: Vec<PayloadFileWritePlan>, owner: &str) -> Result<WritePlan> {
    ensure!(
        plans.len() == 1 && plans[0].file_name == MAD_FILE,
        "{owner} does not own exactly one MAD.COM plan"
    );
    Ok(plans.into_iter().next().expect("one plan was checked").plan)
}
