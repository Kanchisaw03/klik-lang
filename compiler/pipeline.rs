use anyhow::{anyhow, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliOptLevel {
    O0,
    O1,
    O2,
}

impl CliOptLevel {
    pub fn parse(raw: &str) -> Result<Self> {
        match raw.to_ascii_uppercase().as_str() {
            "O0" => Ok(Self::O0),
            "O1" => Ok(Self::O1),
            "O2" => Ok(Self::O2),
            other => Err(anyhow!(
                "Invalid --opt-level '{}'. Supported values: O0, O1, O2",
                other
            )),
        }
    }
}

pub fn run_optimization_pipeline(
    ir_module: &mut klik_ir::IrModule,
    level: CliOptLevel,
    trace: bool,
) -> klik_opt::OptimizeReport {
    match level {
        CliOptLevel::O0 => klik_opt::OptimizeReport::default(),
        CliOptLevel::O1 => {
            let pass = klik_opt::constant_folding(ir_module);
            if trace {
                eprintln!("[OPT] constant folding pass applied ({} folds)", pass.folded);
            }
            klik_opt::OptimizeReport {
                constant_folding: pass,
                ..Default::default()
            }
        }
        CliOptLevel::O2 => {
            let report = klik_opt::optimize(ir_module, klik_opt::OptLevel::O2);
            if trace {
                eprintln!(
                    "[OPT] constant folding pass applied ({} folds)",
                    report.constant_folding.folded
                );
                eprintln!(
                    "[OPT] dead code elimination removed {} instructions",
                    report.dead_code_elimination.removed
                );
                eprintln!(
                    "[OPT] block simplification collapsed {} blocks",
                    report.block_simplification.simplified_blocks
                );
                eprintln!(
                    "[OPT] branch simplification rewrote {} branches",
                    report.branch_simplification.simplified_branches
                );
            }
            report
        }
    }
}
