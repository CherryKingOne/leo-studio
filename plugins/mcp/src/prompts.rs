use rmcp::model::{GetPromptResult, Prompt, PromptArgument, PromptMessage, Role};
use serde_json::Value;

use leo_plugin_api::error::AppError;
use leo_plugin_api::modules::projects::service as project_service;

use super::server::McpHandler;

fn argument(name: &str, description: &str, required: bool) -> PromptArgument {
    let mut item = PromptArgument::new(name);
    item.description = Some(description.to_string());
    item.required = Some(required);
    item
}

/// 工作流提示词清单。
pub fn list() -> Vec<Prompt> {
    vec![
        Prompt::new(
            "leo_project_overview",
            Some("汇总当前剧目库状态，并给出下一步建议。"),
            Some(vec![argument("project_id", "可选：只看某个剧目", false)]),
        ),
        Prompt::new(
            "leo_import_novel",
            Some("把一段小说正文导入为新剧目，并汇报分章结果。"),
            Some(vec![argument("name", "剧目名称", false)]),
        ),
        Prompt::new(
            "leo_chapter_digest",
            Some("阅读指定章节，输出结构化摘要与待确认词。"),
            Some(vec![
                argument("project_id", "剧目 ID", true),
                argument("chapter_id", "章节 ID", true),
            ]),
        ),
    ]
}

fn arg<'a>(args: &'a serde_json::Map<String, Value>, key: &str) -> Option<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn req_arg(args: &serde_json::Map<String, Value>, key: &str) -> Result<String, AppError> {
    arg(args, key)
        .map(str::to_string)
        .ok_or_else(|| AppError::invalid_parameter(format!("提示词缺少参数：{key}")))
}

/// 渲染提示词内容。
pub async fn render(
    handler: &McpHandler,
    name: &str,
    args: &serde_json::Map<String, Value>,
) -> Result<GetPromptResult, AppError> {
    let message = match name {
        "leo_project_overview" => {
            let stats = project_service::get_stats(&*handler.store.lock().await);
            match arg(args, "project_id") {
                Some(project_id) => {
                    let info = project_service::get_project(
                        &*handler.store.lock().await,
                        project_id,
                        false,
                    )?;
                    format!(
                        "当前剧目：{}（{}）\n状态：{}\n章节：{}\n总字数：{}\n创建于：{}\n更新时间：{}\n\n请基于以上信息总结进度，并给出下一步建议（例如追更、标多音字、删改章节）。",
                        info.name,
                        info.project_id,
                        info.status,
                        info.chapter_count,
                        info.total_words,
                        info.created_at,
                        info.updated_at
                    )
                }
                None => format!(
                    "剧目总数：{}\n章节总数：{}\n总字数：{}\n回收站：{}\n\n请概括当前创作进度，并指出值得优先处理的剧目。",
                    stats.projects, stats.chapters, stats.words, stats.recycle_bin
                ),
            }
        }
        "leo_import_novel" => {
            let name = arg(args, "name").unwrap_or("未命名剧目");
            format!(
                "请使用 leo_create_project 新建剧目「{name}」，正文通过 text 参数传入。\n\
                 创建成功后调用 leo_list_chapters 汇报章节数量，并说明清洗与分章结果。\n\
                 如果章节与预期不符，不要直接覆盖；改用 leo_append_chapters 并先以 dry_run=true 预检。"
            )
        }
        "leo_chapter_digest" => {
            let project_id = req_arg(args, "project_id")?;
            let chapter_id = req_arg(args, "chapter_id")?;
            let info = project_service::get_project_chapter_content(
                &*handler.store.lock().await,
                &project_id,
                &chapter_id,
            )?;
            format!(
                "章节：{}（第 {} 章，{} 字，{} 句）\n\n请阅读 leo_get_chapter 返回的正文与逐句，输出：\n\
                 1. 本章剧情摘要（不超过 200 字）；\n\
                 2. 出场人物与关系变化；\n\
                 3. 可能影响配音的多音字/异读词（给出 char_index 与建议拼音）；\n\
                 4. 需要作者确认的疑点。\n\
                 章节 ID：{project_id}/{chapter_id}",
                info.chapter_title,
                info.chapter_index,
                info.word_count,
                info.sentences.len()
            )
        }
        _ => return Err(AppError::file_not_found(format!("提示词不存在：{name}"))),
    };

    Ok(
        GetPromptResult::new(vec![PromptMessage::new_text(Role::User, message)])
            .with_description(format!("Leo Studio 工作流提示词：{name}")),
    )
}
