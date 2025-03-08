use anyhow::{anyhow, Context, Result};
use std::path::Path;
use std::process::Command;
use tracing::{debug, info, warn};

use super::types::ConvertOptions;

#[derive(Debug)]
pub enum ConverterType {
    Document,
    Image,
    Media,
    PDF,
}

pub struct FileConverter {
    libreoffice_available: bool,
    imagemagick_available: bool,
    ffmpeg_available: bool,
    ghostscript_available: bool,
}

impl FileConverter {
    pub fn new() -> Result<Self> {
        // 检查必要工具是否安装
        let libreoffice_available = Command::new("soffice").arg("--version").output().is_ok();
        let imagemagick_available = Command::new("convert").arg("-version").output().is_ok();
        let ffmpeg_available = Command::new("ffmpeg").arg("-version").output().is_ok();
        let ghostscript_available = Command::new("gs").arg("-v").output().is_ok();

        info!(
            "可用的转换工具: LibreOffice={}, ImageMagick={}, FFmpeg={}, Ghostscript={}",
            libreoffice_available, imagemagick_available, ffmpeg_available, ghostscript_available
        );

        Ok(Self {
            libreoffice_available,
            imagemagick_available,
            ffmpeg_available,
            ghostscript_available,
        })
    }

    pub async fn convert(
        &self,
        input: &Path,
        output: &Path,
        options: &ConvertOptions,
    ) -> Result<()> {
        let converter_type = self.detect_converter_type(input, &options.format)?;
        debug!("使用转换器类型: {:?}", converter_type);

        match converter_type {
            ConverterType::Document => self.convert_document(input, output, options).await,
            ConverterType::Image => self.convert_image(input, output, options).await,
            ConverterType::Media => self.convert_media(input, output, options).await,
            ConverterType::PDF => self.convert_pdf(input, output, options).await,
        }
    }

    async fn convert_document(
        &self,
        input: &Path,
        output: &Path,
        _options: &ConvertOptions,
    ) -> Result<()> {
        if !self.libreoffice_available {
            return Err(anyhow!("LibreOffice 未安装，无法进行文档转换"));
        }

        info!("开始转换文档: {:?} -> {:?}", input, output);

        // 使用 LibreOffice 进行转换
        let status = Command::new("soffice")
            .args([
                "--headless",
                "--convert-to",
                output
                    .extension()
                    .and_then(|e| e.to_str())
                    .ok_or_else(|| anyhow!("无效的输出格式"))?,
                input.to_str().unwrap(),
                "--outdir",
                output.parent().unwrap().to_str().unwrap(),
            ])
            .status()
            .context("执行 LibreOffice 转换失败")?;

        if !status.success() {
            return Err(anyhow!("文档转换失败"));
        }

        info!("文档转换完成");
        Ok(())
    }

    async fn convert_image(
        &self,
        input: &Path,
        output: &Path,
        options: &ConvertOptions,
    ) -> Result<()> {
        if !self.imagemagick_available {
            return Err(anyhow!("ImageMagick 未安装，无法进行图片转换"));
        }

        info!("开始转换图片: {:?} -> {:?}", input, output);

        let mut cmd = Command::new("convert");
        cmd.arg(input);

        // 添加质量设置
        if let Some(quality) = &options.quality {
            cmd.args(["-quality", quality]);
            debug!("设置图片质量: {}", quality);
        }

        cmd.arg(output);

        let status = cmd.status().context("执行 ImageMagick 转换失败")?;

        if !status.success() {
            return Err(anyhow!("图片转换失败"));
        }

        info!("图片转换完成");
        Ok(())
    }

    async fn convert_media(
        &self,
        input: &Path,
        output: &Path,
        options: &ConvertOptions,
    ) -> Result<()> {
        if !self.ffmpeg_available {
            return Err(anyhow!("FFmpeg 未安装，无法进行媒体转换"));
        }

        info!("开始转换媒体文件: {:?} -> {:?}", input, output);

        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-i", input.to_str().unwrap()]);

        // 添加质量设置
        if let Some(quality) = &options.quality {
            match quality.as_str() {
                "high" => {
                    cmd.args(["-crf", "18"]);
                    debug!("设置高质量转换");
                }
                "medium" => {
                    cmd.args(["-crf", "23"]);
                    debug!("设置中等质量转换");
                }
                "low" => {
                    cmd.args(["-crf", "28"]);
                    debug!("设置低质量转换");
                }
                _ => {
                    warn!("未知的质量设置: {}, 使用默认值", quality);
                    cmd.args(["-crf", "23"]);
                }
            };
        }

        // 添加额外参数
        if let Some(extra_args) = &options.extra_args {
            cmd.args(extra_args);
            debug!("添加额外参数: {:?}", extra_args);
        }

        cmd.arg(output);

        let status = cmd.status().context("执行 FFmpeg 转换失败")?;

        if !status.success() {
            return Err(anyhow!("媒体转换失败"));
        }

        info!("媒体文件转换完成");
        Ok(())
    }

    async fn convert_pdf(
        &self,
        input: &Path,
        output: &Path,
        _options: &ConvertOptions,
    ) -> Result<()> {
        if !self.ghostscript_available {
            return Err(anyhow!("Ghostscript 未安装，无法进行 PDF 转换"));
        }

        info!("开始转换 PDF: {:?} -> {:?}", input, output);

        let status = Command::new("gs")
            .args([
                "-sDEVICE=pdfwrite",
                "-dNOPAUSE",
                "-dBATCH",
                "-dSAFER",
                &format!("-sOutputFile={}", output.to_str().unwrap()),
                input.to_str().unwrap(),
            ])
            .status()
            .context("执行 Ghostscript 转换失败")?;

        if !status.success() {
            return Err(anyhow!("PDF 转换失败"));
        }

        info!("PDF 转换完成");
        Ok(())
    }

    fn detect_converter_type(&self, input: &Path, target_format: &str) -> Result<ConverterType> {
        let ext = input
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| anyhow!("无法识别文件扩展名"))?
            .to_lowercase();

        debug!(
            "检测文件类型: 扩展名 = {}, 目标格式 = {}",
            ext, target_format
        );

        match ext.as_str() {
            // 文档格式
            "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods" | "odp" => {
                Ok(ConverterType::Document)
            }
            // 图片格式
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "tiff" => Ok(ConverterType::Image),
            // 媒体格式
            "mp4" | "avi" | "mkv" | "mov" | "mp3" | "wav" | "flac" => Ok(ConverterType::Media),
            // PDF 相关
            "pdf" | "ps" | "eps" => Ok(ConverterType::PDF),
            _ => Err(anyhow!("不支持的文件格式: {}", ext)),
        }
    }
}
