package com.wmdb.service;

import io.minio.MinioClient;
import io.minio.PutObjectArgs;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.stereotype.Service;
import org.springframework.web.multipart.MultipartFile;

import jakarta.annotation.PostConstruct;
import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.UUID;

/**
 * 存储服务
 * <p>
 * 负责处理 SQL 脚本文件的存储。实现长短文分流机制：
 * 小文件直接转文本，超大文件上传至 MinIO 并截取摘要防 OOM。
 * </p>
 *
 * @author wm
 * @date 2023-10-25
 */
@Service
public class StorageService {

    /**
     * MinIO 客户端
     */
    private MinioClient minioClient;

    @Value("${wmdb.minio.endpoint}")
    private String endpoint;

    @Value("${wmdb.minio.access-key}")
    private String accessKey;

    @Value("${wmdb.minio.secret-key}")
    private String secretKey;

    @Value("${wmdb.minio.bucket}")
    private String bucketName;

    /**
     * 初始化 MinIO 客户端
     */
    @PostConstruct
    public void init() {
        this.minioClient = MinioClient.builder()
                .endpoint(endpoint)
                .credentials(accessKey, secretKey)
                .build();
    }

    /**
     * 获取 MinIO 客户端实例
     * @return MinioClient
     */
    public MinioClient getMinioClient() {
        return minioClient;
    }

    /**
     * 获取 Bucket Name
     * @return 存储桶名称
     */
    public String getBucketName() {
        return bucketName;
    }

    /**
     * 处理上传的 SQL 文件
     *
     * @param file 上传的文件对象
     * @return 存储结果封装（含 SQL 文本或 OSS Key）
     * @throws Exception 上传或读取异常
     */
    public StorageResult processSqlFile(MultipartFile file) throws Exception {
        long size = file.getSize();
        StorageResult result = new StorageResult();

        if (size <= 1024 * 1024) { // <= 1MB
            String content = new String(file.getBytes(), StandardCharsets.UTF_8);
            result.setSqlText(content);
            result.setAstCheckText(content);
            result.setAttachmentOssKey(null);
        } else { // > 1MB
            // Upload to MinIO
            String objectKey = UUID.randomUUID().toString() + "-" + file.getOriginalFilename();
            minioClient.putObject(
                    PutObjectArgs.builder()
                            .bucket(bucketName)
                            .object(objectKey)
                            .stream(file.getInputStream(), size, -1)
                            .contentType(file.getContentType())
                            .build()
            );

            // Extract 50 lines for summary, 100 lines for AST parse
            StringBuilder summary = new StringBuilder();
            StringBuilder astText = new StringBuilder();
            try (BufferedReader reader = new BufferedReader(new InputStreamReader(file.getInputStream(), StandardCharsets.UTF_8))) {
                String line;
                int count = 0;
                while ((line = reader.readLine()) != null && count < 100) {
                    if (count < 50) {
                        summary.append(line).append("\n");
                    }
                    astText.append(line).append("\n");
                    count++;
                }
            }
            summary.append("...[内容过长已截断]");

            result.setSqlText(summary.toString());
            result.setAstCheckText(astText.toString());
            result.setAttachmentOssKey(objectKey);
        }

        return result;
    }

    /**
     * 存储结果内部类
     */
    public static class StorageResult {
        /**
         * SQL 文本（全量或摘要）
         */
        private String sqlText;

        /**
         * 专用于 AST 预检的 100 行有效 SQL（无后缀）
         */
        private String astCheckText;

        /**
         * 附件在 MinIO 中的 Key
         */
        private String attachmentOssKey;

        public String getSqlText() {
            return sqlText;
        }

        public void setSqlText(String sqlText) {
            this.sqlText = sqlText;
        }

        public String getAstCheckText() {
            return astCheckText;
        }

        public void setAstCheckText(String astCheckText) {
            this.astCheckText = astCheckText;
        }

        public String getAttachmentOssKey() {
            return attachmentOssKey;
        }

        public void setAttachmentOssKey(String attachmentOssKey) {
            this.attachmentOssKey = attachmentOssKey;
        }
    }
}
