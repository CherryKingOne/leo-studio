use serde::Deserialize;

use crate::error::{AppError, AppResult};

/// 通用分页查询参数。
#[derive(Debug, Clone, Deserialize)]
pub struct Pagination {
    pub page: Option<usize>,
    pub page_size: Option<usize>,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            page: Some(1),
            page_size: Some(20),
        }
    }
}

impl Pagination {
    /// 校验并返回 (page, page_size)。
    pub fn resolve(&self) -> AppResult<(usize, usize)> {
        let page = self.page.unwrap_or(1);
        let page_size = self.page_size.unwrap_or(20);
        if page < 1 {
            return Err(AppError::invalid_parameter("页码必须大于 0"));
        }
        if !(1..=100).contains(&page_size) {
            return Err(AppError::invalid_parameter("每页数量必须在 1 到 100 之间"));
        }
        Ok((page, page_size))
    }
}

/// 计算总页数（无数据时为 0）。
pub fn page_count(total: usize, page_size: usize) -> usize {
    if total == 0 {
        0
    } else {
        total.div_ceil(page_size)
    }
}
