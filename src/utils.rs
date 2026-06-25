/// 从markdown内容中提取mermaid代码块
/// 返回 (开始位置, 结束位置, 代码内容) 的元组列表
pub fn extract_mermaid_blocks(content: &str) -> Vec<(usize, usize, String)> {
    let mut blocks = Vec::new();
    
    // 直接在原始字符串中搜索 ```mermaid 标记，避免 lines() 去除行尾符导致偏移计算错误
    let mut search_start = 0;
    while let Some(block_start) = content[search_start..].find("```mermaid") {
        let abs_start = search_start + block_start;
        
        // 找到块起始后的第一个换行符作为代码的起始
        let code_start = match content[abs_start..].find('\n') {
            Some(pos) => abs_start + pos + 1,
            None => break,
        };
        
        // 从 abs_start 之后找 "```" 作为块结束
        let after_block_start = abs_start + 8; // "```mermaid" 长度
        if let Some(end_marker) = content[after_block_start..].find("```") {
            let abs_end_marker = after_block_start + end_marker;
            
            // 收集代码内容：从 code_start 到 abs_end_marker
            let mermaid_code = content[code_start..abs_end_marker].to_string();
            
            // end_pos 需要包含结束 ``` 所在的行尾符
            let block_end = match content[abs_end_marker..].find('\n') {
                Some(pos) => abs_end_marker + pos + 1,
                None => content.len(),
            };
            
            blocks.push((abs_start, block_end, mermaid_code));
            search_start = block_end;
        } else {
            search_start += 8;
        }
    }

    blocks
}

/// 打印统计信息
pub fn print_statistics(result: &crate::processor::ProcessResult, dry_run: bool) {
    println!("\n📊 处理完成:");
    println!("   📄 处理文件数: {}", result.total_files);
    println!("   📊 总mermaid代码块数: {}", result.total_mermaid_blocks);
    println!("   ❌ 无效代码块数: {}", result.invalid_blocks);
    if !dry_run {
        println!("   🔧 成功修复数: {}", result.fixed_blocks);
    }
}
