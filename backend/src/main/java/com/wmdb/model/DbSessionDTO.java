package com.wmdb.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

/**
 * 数据库活动会话 Process DTO
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class DbSessionDTO {

    /**
     * 会话 ID / 进程 ID
     */
    private Long id;

    /**
     * 连接用户
     */
    private String user;

    /**
     * 客户端主机 IP 与端口
     */
    private String host;

    /**
     * 当前选中的数据库
     */
    private String db;

    /**
     * 执行命令类型 (如 Query, Sleep, Execute 等)
     */
    private String command;

    /**
     * 当前状态持续时间 (秒)
     */
    private Long time;

    /**
     * 会话状态描述 (如 Sending data, Locked, executing 等)
     */
    private String state;

    /**
     * 正在执行的 SQL 语句文本
     */
    private String info;
}
