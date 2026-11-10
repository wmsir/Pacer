package com.wmdb.model;

import com.baomidou.mybatisplus.annotation.IdType;
import com.baomidou.mybatisplus.annotation.TableId;
import com.baomidou.mybatisplus.annotation.TableName;
import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

import java.util.Date;

/**
 * 业务资源组实体类
 * <p>
 * 映射 resource_group 表，用于资产隔离与审批流智能路由。
 * </p>
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
@TableName("resource_group")
public class ResourceGroup {

    private String tenantId;

    /**
     * 资源组 ID
     */
    @TableId(type = IdType.AUTO)
    private Long id;

    /**
     * 资源组名称（如：车险承保资源组）
     */
    private String groupName;

    /**
     * 所属部门 / 业务条线
     */
    private String deptName;

    /**
     * 开发组长 (初审负责人) 用户名或姓名
     */
    private String devLead;

    /**
     * DBA 负责人
     */
    private String dbaLead;

    /**
     * 描述说明
     */
    private String description;

    /**
     * 绑定的审批流程列表 (JSON 数组字符串，如 '["标准 DML 常规两级审批流", "高危 DDL 结构变更三级严格审批流"]')
     */
    private String workflowTemplates;

    /**
     * 工单可配置化字段规则 (JSON 字符串，定义该资源组下工单字段的启用状态、必填属性与格式)
     */
    private String formConfig;

    /**
     * 状态（1-正常启用，0-禁用）
     */
    private Integer status;

    /**
     * 创建时间
     */
    private Date createTime;
}
