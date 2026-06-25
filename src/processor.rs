use std::path::PathBuf;

use crate::config::Config;
use crate::markdown_scanner::MarkdownScanner;
use crate::mermaid_validator::MermaidValidator;
use crate::ai_fixer::AiFixer;
use crate::utils::extract_mermaid_blocks;

pub struct MermaidProcessor {
    scanner: MarkdownScanner,
    ai_fixer: Option<AiFixer>,
    verbose: bool,
}

pub struct ProcessResult {
    pub total_files: usize,
    pub total_mermaid_blocks: usize,
    pub invalid_blocks: usize,
    pub fixed_blocks: usize,
}

impl MermaidProcessor {
    pub async fn new(config: &Config, dry_run: bool, verbose: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let scanner = MarkdownScanner::new();
        
        
        let ai_fixer = if dry_run {
            None
        } else {
            Some(AiFixer::new(config).await?)
        };

        Ok(Self {
            scanner,
            ai_fixer,
            verbose,
        })
    }

    pub async fn process_directory(&self, directory: PathBuf, dry_run: bool) -> Result<ProcessResult, Box<dyn std::error::Error>> {
        if self.verbose {
            println!("🚀 开始扫描目录: {}", directory.display());
        }
        
        // 扫描markdown文件
        let markdown_files = self.scanner.scan_directory(&directory)?;
        
        if self.verbose {
            println!("📄 找到 {} 个markdown文件", markdown_files.len());
        }

        let mut total_mermaid_blocks = 0;
        let mut invalid_blocks = 0;
        let mut fixed_blocks = 0;

        // 处理每个markdown文件
        for file_path in &markdown_files {
            if self.verbose {
                println!("\n📝 处理文件: {}", file_path.display());
            }
            
            let result = self.process_file(file_path, dry_run).await?;
            
            total_mermaid_blocks += result.total_blocks;
            invalid_blocks += result.invalid_blocks;
            fixed_blocks += result.fixed_blocks;
        }

        Ok(ProcessResult {
            total_files: markdown_files.len(),
            total_mermaid_blocks,
            invalid_blocks,
            fixed_blocks,
        })
    }

    async fn process_file(&self, file_path: &PathBuf, dry_run: bool) -> Result<FileProcessResult, Box<dyn std::error::Error>> {
        // 打印正在处理的文件名（无论 verbose 与否）
        if !self.verbose {
            println!("📝 正在修复: {}", file_path.display());
        }
        // 读取文件内容
        let content = std::fs::read_to_string(file_path)?;

        // 提取mermaid代码块
        let mermaid_blocks = extract_mermaid_blocks(&content);
        
        if mermaid_blocks.is_empty() {
            if self.verbose {
                println!("   ℹ️  未找到mermaid代码块");
            }
            return Ok(FileProcessResult::default());
        }

        if self.verbose {
            println!("   🔍 找到 {} 个mermaid代码块", mermaid_blocks.len());
        }

        let mut invalid_blocks = 0;
        let mut fixed_blocks = 0;
        let mut file_modified = false;
        let mut new_content = content.clone();

        // 验证每个mermaid代码块
        // 从后往前遍历，避免前面替换导致位置偏移
        for (index, (start_pos, end_pos, mermaid_code)) in mermaid_blocks.iter().enumerate().rev() {
            if self.verbose {
                println!("      📊 验证代码块 {}/{}", index + 1, mermaid_blocks.len());
            }
            
            let validator = MermaidValidator::with_config(None)?;
            
            match validator.validate(mermaid_code) {
                Ok(_) => {
                    if self.verbose {
                        println!("         ✅ 代码块有效");
                    }
                }
                Err(e) => {
                    if self.verbose {
                        println!("         ❌ 代码块无效: {}", e);
                    }
                    invalid_blocks += 1;

                    if !dry_run {
                        if let Some(ai_fixer) = &self.ai_fixer {
                            match ai_fixer.fix_mermaid(mermaid_code).await {
                                Ok(fixed_code) => {
                                    // 验证修复后的代码
                                    let validator = MermaidValidator::with_config(None)?;
                                    match validator.validate(&fixed_code) {
                                        Ok(_) => {
                                            if self.verbose {
                                                println!("         🔧 修复成功: {}", file_path.display());
                                            } else {
                                                println!("   ✅ 修复成功: {}", file_path.display());
                                            }
                                            // 精确按字节位置替换整个代码块（包含```mermaid和```标记）
                                            let original_block = &new_content[*start_pos..*end_pos];
                                            let fixed_block = format!("```mermaid\n{}\n```", fixed_code);
                                            if original_block.len() == fixed_block.len() {
                                                // 等长替换，安全直接覆盖
                                                unsafe {
                                                    let bytes = new_content.as_bytes_mut();
                                                    bytes[*start_pos..*end_pos].copy_from_slice(fixed_block.as_bytes());
                                                }
                                            } else {
                                                // 不等长，重建字符串
                                                let prefix = &new_content[..*start_pos];
                                                let suffix = &new_content[*end_pos..];
                                                new_content = format!("{}{}{}", prefix, fixed_block, suffix);
                                            }
                                            file_modified = true;
                                            fixed_blocks += 1;
                                        }
                                        Err(validation_error) => {
                                            if self.verbose {
                                                println!("         ⚠️  修复后的代码仍然无效: {}", validation_error);
                                            }
                                        }
                                    }
                                }
                                Err(fix_error) => {
                                    if self.verbose {
                                        println!("         ⚠️  AI修复失败: {}", fix_error);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 如果文件被修改，写回文件
        if file_modified {
            std::fs::write(file_path, new_content)?;
            if self.verbose {
                println!("   💾 文件已更新: {}", file_path.display());
            }
        }

        Ok(FileProcessResult {
            total_blocks: mermaid_blocks.len(),
            invalid_blocks,
            fixed_blocks,
        })
    }
}

#[derive(Default)]
struct FileProcessResult {
    total_blocks: usize,
    invalid_blocks: usize,
    fixed_blocks: usize,
}