use async_trait::async_trait;

use crate::context::Context;
use crate::error::CoreResult;
use crate::plugin::manifest::PluginManifest;

/// 插件根接口：参考 Cordis / leo-agents 的 apply(ctx) 哲学。
///
/// `apply` 收到的是**插件作用域**的 [`Context`]：通过 `ctx.register(...)` 往注册表里注册的驱动、
/// 通过 `ctx.on_dispose(...)` 登记的清理函数，都会在插件卸载（或装配失败回滚）时自动撤销。
#[async_trait]
pub trait Plugin: Send + Sync {
    /// 插件的元数据清单
    fn manifest(&self) -> PluginManifest;

    /// 挂载插件：在此方法内向 Context 注册驱动 / 服务、监听事件
    async fn apply(&self, ctx: &Context) -> CoreResult<()>;
}
