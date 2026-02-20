//! 沙箱执行器

use crate::god_mode::{GodModeAction, GodModeExecutor};
use crate::micro_vm::{ExecutionResult, MicroVM};

/// 沙箱执行器
pub struct SandboxExecutor {
    /// God Mode 执行器
    god_mode: GodModeExecutor,
    /// 微型虚拟机池
    vm_pool: Vec<MicroVM>,
}

impl SandboxExecutor {
    /// 创建新执行器
    pub fn new() -> Self {
        Self {
            god_mode: GodModeExecutor::new(),
            vm_pool: Vec::new(),
        }
    }

    /// 执行 God Mode 操作
    pub async fn execute_god_mode(&self, action: GodModeAction) -> nl_core::Result<crate::god_mode::GodModeResult> {
        self.god_mode.execute(action).await
    }

    /// 在隔离环境中执行代码
    pub async fn execute_isolated(&self, code: &str, language: &str) -> nl_core::Result<ExecutionResult> {
        let vm = MicroVM::default_vm();
        vm.execute(code, language).await
    }

    /// 添加虚拟机到池
    pub fn add_vm(&mut self, vm: MicroVM) {
        self.vm_pool.push(vm);
    }

    /// 获取虚拟机数量
    pub fn vm_count(&self) -> usize {
        self.vm_pool.len()
    }
}

impl Default for SandboxExecutor {
    fn default() -> Self {
        Self::new()
    }
}
