use std::path::PathBuf;

use strum::EnumString;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpTopic {
    Hni,
    Ni,
    Nr,
    Nlx,
    Nru,
    Nun,
    Nci,
    Na,
    Np,
    Ns,
    Node,
    Init,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvocationKind {
    Hni,
    Ni,
    Nr,
    Nlx,
    Nru,
    Nun,
    Nci,
    Na,
    Np,
    Ns,
    NodeShim,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Install,
    Add,
    Run,
    Execute,
    Upgrade,
    Uninstall,
    CleanInstall,
    AgentAlias,
    PassthroughNode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedExecution {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub passthrough: bool,
    pub mode: ExecutionMode,
    pub strategy: ExecutionStrategy,
    pub fast_requested: bool,
    pub fast_fallback_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    PackageManager,
    NodeRun,
    PassthroughNode,
    Fast,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionStrategy {
    External,
    Native(NativeExecution),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeExecution {
    RunScript(NativeScriptExecution),
    RunDenoTask(NativeDenoTaskExecution),
    RunLocalBin(NativeLocalBinExecution),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeScriptExecution {
    pub package_root: PathBuf,
    pub package_json_path: PathBuf,
    pub script_name: String,
    pub steps: Vec<NativeScriptStep>,
    pub forwarded_args: Vec<String>,
    pub bin_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeScriptStep {
    pub event_name: String,
    pub command: String,
    pub forward_args: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDenoTaskExecution {
    pub project_root: PathBuf,
    pub config_path: Option<PathBuf>,
    pub selection: String,
    pub stages: Vec<NativeDenoTaskStage>,
    pub forwarded_args: Vec<String>,
    pub bin_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDenoTaskStage {
    pub steps: Vec<NativeDenoTaskStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDenoTaskStep {
    pub task_name: String,
    pub command: String,
    pub forward_args: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeLocalBinExecution {
    pub bin_name: String,
    pub launcher: NativeLocalBinLauncher,
    pub forwarded_args: Vec<String>,
    pub bin_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeLocalBinLauncher {
    Binary(PathBuf),
    Cmd(PathBuf),
    PowerShell(PathBuf),
    NodeScript {
        script_path: PathBuf,
        node_args: Vec<String>,
    },
}

impl NativeLocalBinExecution {
    pub fn resolved_path(&self) -> &PathBuf {
        match &self.launcher {
            NativeLocalBinLauncher::Binary(path)
            | NativeLocalBinLauncher::Cmd(path)
            | NativeLocalBinLauncher::PowerShell(path) => path,
            NativeLocalBinLauncher::NodeScript { script_path, .. } => script_path,
        }
    }
}

impl ResolvedExecution {
    pub fn external(
        program: impl Into<String>,
        args: Vec<String>,
        cwd: PathBuf,
        passthrough: bool,
    ) -> Self {
        Self::external_with_mode(
            program,
            args,
            cwd,
            passthrough,
            ExecutionMode::PackageManager,
        )
    }

    pub fn external_with_mode(
        program: impl Into<String>,
        args: Vec<String>,
        cwd: PathBuf,
        passthrough: bool,
        mode: ExecutionMode,
    ) -> Self {
        Self {
            program: program.into(),
            args,
            cwd,
            passthrough,
            mode,
            strategy: ExecutionStrategy::External,
            fast_requested: false,
            fast_fallback_reason: None,
        }
    }

    pub fn external_with_native_fallback(
        program: impl Into<String>,
        args: Vec<String>,
        cwd: PathBuf,
        passthrough: bool,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            program: program.into(),
            args,
            cwd,
            passthrough,
            mode: ExecutionMode::PackageManager,
            strategy: ExecutionStrategy::External,
            fast_requested: true,
            fast_fallback_reason: Some(reason.into()),
        }
    }

    pub fn native_script(
        script_name: impl Into<String>,
        cwd: PathBuf,
        exec: NativeScriptExecution,
    ) -> Self {
        let script_name = script_name.into();
        Self {
            program: script_name.clone(),
            args: exec.forwarded_args.clone(),
            cwd,
            passthrough: false,
            mode: ExecutionMode::Fast,
            strategy: ExecutionStrategy::Native(NativeExecution::RunScript(exec)),
            fast_requested: true,
            fast_fallback_reason: None,
        }
    }

    pub fn native_deno_task(
        selection: impl Into<String>,
        cwd: PathBuf,
        exec: NativeDenoTaskExecution,
    ) -> Self {
        let selection = selection.into();
        Self {
            program: selection.clone(),
            args: exec.forwarded_args.clone(),
            cwd,
            passthrough: false,
            mode: ExecutionMode::Fast,
            strategy: ExecutionStrategy::Native(NativeExecution::RunDenoTask(exec)),
            fast_requested: true,
            fast_fallback_reason: None,
        }
    }

    pub fn native_local_bin(
        bin_name: impl Into<String>,
        cwd: PathBuf,
        exec: NativeLocalBinExecution,
    ) -> Self {
        Self {
            program: bin_name.into(),
            args: exec.forwarded_args.clone(),
            cwd,
            passthrough: false,
            mode: ExecutionMode::Fast,
            strategy: ExecutionStrategy::Native(NativeExecution::RunLocalBin(exec)),
            fast_requested: true,
            fast_fallback_reason: None,
        }
    }

    pub fn is_native(&self) -> bool {
        matches!(self.strategy, ExecutionStrategy::Native(_))
    }

    pub fn execution_mode_name(&self) -> &'static str {
        match self.mode {
            ExecutionMode::PackageManager => "package-manager",
            ExecutionMode::NodeRun => "node-run",
            ExecutionMode::PassthroughNode => "passthrough-node",
            ExecutionMode::Fast => "fast",
            ExecutionMode::Internal => "internal",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeShimMode {
    RouteToIntent(Intent),
    RunParallel,
    RunSequential,
    PassthroughNode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeShimDecision {
    pub mode: NodeShimMode,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum PackageManager {
    Npm,
    Yarn,
    #[strum(serialize = "yarn-berry", serialize = "yarnberry")]
    YarnBerry,
    Pnpm,
    Bun,
    Deno,
}

impl PackageManager {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Npm => "npm",
            Self::Yarn => "yarn",
            Self::YarnBerry => "yarn (berry)",
            Self::Pnpm => "pnpm",
            Self::Bun => "bun",
            Self::Deno => "deno",
        }
    }

    pub fn bin(self) -> &'static str {
        match self {
            Self::Npm => "npm",
            Self::Yarn | Self::YarnBerry => "yarn",
            Self::Pnpm => "pnpm",
            Self::Bun => "bun",
            Self::Deno => "deno",
        }
    }

    pub fn global_package_name(self) -> &'static str {
        self.bin()
    }

    pub fn from_name(value: &str) -> Option<Self> {
        value.parse().ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionSource {
    PackageManagerField,
    Lockfile,
    DevEnginesField,
    InstallMetadata,
    Config,
    Fallback,
    None,
}

#[derive(Debug, Clone)]
pub struct DetectionResult {
    pub agent: Option<PackageManager>,
    pub has_lock: bool,
    pub version_hint: Option<String>,
    pub source: DetectionSource,
}
