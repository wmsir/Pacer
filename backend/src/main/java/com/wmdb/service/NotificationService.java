package com.wmdb.service;

import com.baomidou.mybatisplus.core.conditions.query.LambdaQueryWrapper;
import com.baomidou.mybatisplus.core.conditions.query.QueryWrapper;
import com.wmdb.mapper.SysUserMapper;
import com.wmdb.mapper.TicketOperationLogMapper;
import com.wmdb.model.NotificationConfigDTO;
import com.wmdb.model.SqlTicket;
import com.wmdb.model.SysUser;
import com.wmdb.model.TicketOperationLog;
import lombok.Data;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Service;
import org.springframework.util.StringUtils;

import java.io.BufferedReader;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.util.*;

/**
 * 企业级消息通知服务（对接企业微信 WebService / WSDL 消息总线、阿里钉钉、字节飞书与紧急电话语音外呼）
 *
 * 特性：
 * 1. 工单各审批节点流转进度通知（推送给工单创建人，消息话术清晰体现多级审批链条）
 * 2. 审批前默认仅推送 1 次给当前节点待审批人
 * 3. 支持页面一键加急「催办」多通道再次推送
 *
 * @author wm
 */
@Slf4j
@Service
@RequiredArgsConstructor
public class NotificationService {

    private final SysUserMapper sysUserMapper;
    private final TicketOperationLogMapper ticketOperationLogMapper;
    private final NotificationConfigService notificationConfigService;

    /**
     * 发送工单生命周期流转通知 (通用接口)
     */
    public void sendTicketNotification(SqlTicket ticket, String status) {
        sendApprovalStageNotification(ticket, status, null, null, null);
    }

    /**
     * 发送节点审批流转通知 (通知工单创建人最新进度与审批流链条，并通知下一节点待审批人)
     *
     * @param ticket 工单对象
     * @param status 目标状态 (SUBMITTED, AUDITING, APPROVED, REJECTED, EXECUTED, FAILED 等)
     * @param approvedNodeName 刚刚通过的节点名称 (如 "业务开发组长初审")
     * @param approverName 刚刚执行审核的人员姓名
     * @param comment 审批批注意见
     */
    public void sendApprovalStageNotification(SqlTicket ticket, String status, String approvedNodeName, String approverName, String comment) {
        if (ticket == null) return;
        log.info("[消息推送] 工单 #{} 触发审批流转通知: status={}, approvedNode={}", ticket.getId(), status, approvedNodeName);

        try {
            NotificationConfigDTO config = notificationConfigService.getRawConfig();
            NotificationConfigDTO.PolicyConfig policy = config != null ? config.getPolicy() : null;

            // 策略过滤
            if (policy != null) {
                if ("SUBMITTED".equalsIgnoreCase(status) && !Boolean.TRUE.equals(policy.getNotifyOnSubmit())) return;
                if (("APPROVED".equalsIgnoreCase(status) || "REJECTED".equalsIgnoreCase(status) || "AUDITING".equalsIgnoreCase(status))
                        && !Boolean.TRUE.equals(policy.getNotifyOnAudited())) return;
                if ("EXECUTED".equalsIgnoreCase(status) && !Boolean.TRUE.equals(policy.getNotifyOnExecuted())) return;
                if ("FAILED".equalsIgnoreCase(status) && !Boolean.TRUE.equals(policy.getNotifyOnFailed())) return;
            }

            // 1. 构建包含审批链条进度的富文本 Markdown 消息卡片
            String markdownContent = buildApprovalStageMarkdownMessage(ticket, status, approvedNodeName, approverName, comment);
            String title = "【wmDB 工单" + formatStatusZh(status) + "】#" + ticket.getId();

            // 2. 企业微信推送：精准推送给工单创建人 (申请人)
            if (config != null && config.getWechat() != null && Boolean.TRUE.equals(config.getWechat().getEnabled())) {
                String applicantErp = resolveApplicantErp(ticket);
                if (StringUtils.hasText(applicantErp)) {
                    log.info("[企微推送] 向工单创建人 [{}] 推送审批进度通知", applicantErp);
                    sendMessage(applicantErp, markdownContent);
                }

                // 若处于审批流转中 (SUBMITTED 或 AUDITING)，默认给当前节点的待审批人也推送 1 次待办提醒
                if ("SUBMITTED".equalsIgnoreCase(status) || "AUDITING".equalsIgnoreCase(status)) {
                    List<String> pendingApproverErps = resolvePendingApproverErps(ticket);
                    if (pendingApproverErps != null) {
                        for (String approverErp : pendingApproverErps) {
                            if (StringUtils.hasText(approverErp) && !approverErp.equalsIgnoreCase(applicantErp)) {
                                String pendingMsg = buildApproverPendingMarkdownMessage(ticket, markdownContent);
                                sendMessage(approverErp, pendingMsg);
                            }
                        }
                    }
                }
            }

            // 3. 钉钉群广播
            if (config != null && config.getDingtalk() != null && Boolean.TRUE.equals(config.getDingtalk().getEnabled())) {
                notificationConfigService.sendDingtalkMessage(title, markdownContent);
            }

            // 4. 飞书群互动卡片广播
            if (config != null && config.getFeishu() != null && Boolean.TRUE.equals(config.getFeishu().getEnabled())) {
                notificationConfigService.sendFeishuMessage(title, markdownContent, status, ticket.getId());
            }

            // 5. 紧急电话语音外呼（若执行失败或被高危拦截）
            if ("FAILED".equalsIgnoreCase(status) && config != null && config.getVoiceCall() != null && Boolean.TRUE.equals(config.getVoiceCall().getEnabled())) {
                String phone = resolveRecipientPhone(ticket);
                if (StringUtils.hasText(phone)) {
                    notificationConfigService.triggerEmergencyVoiceCall(phone, "工单 #" + ticket.getId() + " 执行失败告警");
                }
            }
        } catch (Exception e) {
            log.error("[消息推送] 发送工单 #{} 审批流转通知异常: {}", ticket.getId(), e.getMessage(), e);
        }
    }

    /**
     * 发送加急「催办」通知给当前节点的所有待审批人
     *
     * @param ticket 工单对象
     * @param urgerName 催办发起人姓名
     * @param urgeReason 催办说明 (如 "生产上线窗口临近，劳烦领导尽快协助审批")
     */
    public void sendUrgeNotification(SqlTicket ticket, String urgerName, String urgeReason) {
        if (ticket == null) return;
        log.info("[消息推送] 工单 #{} 收到申请人 [{}] 的加急催办请求", ticket.getId(), urgerName);

        try {
            NotificationConfigDTO config = notificationConfigService.getRawConfig();

            // 1. 构建催办专属加急卡片
            String urgeMarkdown = buildUrgeMarkdownMessage(ticket, urgerName, urgeReason);
            String title = "【⏰ 加急催办】wmDB 变更工单 #" + ticket.getId() + " 待您审批";

            // 2. 企业微信：向所有当前节点待审批人 ERP 发送加急提醒
            if (config != null && config.getWechat() != null && Boolean.TRUE.equals(config.getWechat().getEnabled())) {
                List<String> pendingApproverErps = resolvePendingApproverErps(ticket);
                if (pendingApproverErps != null && !pendingApproverErps.isEmpty()) {
                    for (String approverErp : pendingApproverErps) {
                        if (StringUtils.hasText(approverErp)) {
                            sendMessage(approverErp, urgeMarkdown);
                        }
                    }
                }
            }

            // 3. 钉钉群广播催办提醒
            if (config != null && config.getDingtalk() != null && Boolean.TRUE.equals(config.getDingtalk().getEnabled())) {
                notificationConfigService.sendDingtalkMessage(title, urgeMarkdown);
            }

            // 4. 飞书群广播红色加急互动卡片
            if (config != null && config.getFeishu() != null && Boolean.TRUE.equals(config.getFeishu().getEnabled())) {
                notificationConfigService.sendFeishuMessage(title, urgeMarkdown, "REJECTED", ticket.getId());
            }
        } catch (Exception e) {
            log.error("[消息推送] 发送工单 #{} 加急催办通知异常: {}", ticket.getId(), e.getMessage(), e);
        }
    }

    /**
     * 构建带有清晰审批流链条的 Markdown 消息
     */
    private String buildApprovalStageMarkdownMessage(SqlTicket ticket, String status, String approvedNodeName, String approverName, String comment) {
        String statusZh = formatStatusZh(status);
        String typeDesc = ticket.getType() != null ? ticket.getType() : "SQL 变更工单";
        String dbDesc = ticket.getDbName() != null ? ticket.getDbName() : "默认库";
        String applicantDesc = ticket.getApplicantName() != null ? ticket.getApplicantName() : (ticket.getApplicantIdCard() != null ? ticket.getApplicantIdCard() : "申请人");

        StringBuilder sb = new StringBuilder();
        sb.append("### 【wmDB 工单变更进度提醒】\n");
        sb.append("> **工单编号**：#").append(ticket.getId()).append(" (").append(ticket.getBusinessKey() != null ? ticket.getBusinessKey() : "-").append(")\n");
        sb.append("> **工单类型**：<font color=\"comment\">").append(typeDesc).append("</font>\n");
        sb.append("> **目标数据库**：<font color=\"info\">").append(dbDesc).append("</font>\n");
        sb.append("> **申请人**：`").append(applicantDesc).append("`\n");
        sb.append("> **当前工单状态**：<font color=\"").append(getStatusColor(status)).append("\">").append(statusZh).append("</font>\n");

        if (StringUtils.hasText(approvedNodeName)) {
            sb.append("> **最新通过节点**：<font color=\"info\">【").append(approvedNodeName).append("】已通过</font>\n");
            if (StringUtils.hasText(approverName)) {
                sb.append("> **审核人**：`").append(approverName).append("`\n");
            }
            if (StringUtils.hasText(comment)) {
                sb.append("> **审批批注**：<font color=\"comment\">").append(comment).append("</font>\n");
            }
        }

        // 生成可视化审批流程进度链条
        String workflowChain = buildWorkflowChainVisualization(ticket, status);
        sb.append(">\n");
        sb.append("> ⛓️ **当前审批流全流程进度**：\n");
        sb.append(workflowChain).append("\n");

        sb.append("请登录 [wmDB 完美数据库平台](http://localhost:5173/ticket/").append(ticket.getId()).append(") 查看详情与跟踪进度。");
        return sb.toString();
    }

    /**
     * 构建加急催办专属 Markdown 消息卡片
     */
    private String buildUrgeMarkdownMessage(SqlTicket ticket, String urgerName, String urgeReason) {
        String typeDesc = ticket.getType() != null ? ticket.getType() : "SQL 变更工单";
        String dbDesc = ticket.getDbName() != null ? ticket.getDbName() : "默认库";
        String applicantDesc = ticket.getApplicantName() != null ? ticket.getApplicantName() : (ticket.getApplicantIdCard() != null ? ticket.getApplicantIdCard() : "申请人");
        String reasonText = StringUtils.hasText(urgeReason) ? urgeReason : "生产变更窗口临近，请审批人尽快协助审核";

        StringBuilder sb = new StringBuilder();
        sb.append("### 【⏰ wmDB 变更工单加急催办提醒】\n");
        sb.append("> **工单编号**：#").append(ticket.getId()).append(" (").append(ticket.getBusinessKey() != null ? ticket.getBusinessKey() : "-").append(")\n");
        sb.append("> **工单类型**：<font color=\"comment\">").append(typeDesc).append("</font>\n");
        sb.append("> **目标数据库**：<font color=\"info\">").append(dbDesc).append("</font>\n");
        sb.append("> **工单申请人**：`").append(applicantDesc).append("`\n");
        sb.append("> **催办发起人**：`").append(StringUtils.hasText(urgerName) ? urgerName : applicantDesc).append("`\n");
        sb.append("> **加急催办说明**：<font color=\"warning\"><b>").append(reasonText).append("</b></font>\n");
        sb.append(">\n");
        sb.append("> ⛓️ **审批流流转状态**：\n");
        sb.append(buildWorkflowChainVisualization(ticket, ticket.getStatus())).append("\n");

        sb.append("请审核人尽快点击 [👉 立即进入工单审批](http://localhost:5173/ticket/").append(ticket.getId()).append(") 进行处理。");
        return sb.toString();
    }

    /**
     * 构建待审批人待办提示
     */
    private String buildApproverPendingMarkdownMessage(SqlTicket ticket, String baseMarkdown) {
        return "### 【🔔 您有一条待审批的 SQL 工单待处理】\n" +
                "> 请及时登录系统处理审批流程。\n\n" +
                baseMarkdown;
    }

    /**
     * 生成结构化、可视化的审批流多节点进度链条 (话术体现审批流)
     */
    private String buildWorkflowChainVisualization(SqlTicket ticket, String status) {
        String tplName = ticket.getWorkflowTemplateName() != null ? ticket.getWorkflowTemplateName() : "";
        boolean is4Level = tplName.contains("四级");
        boolean is3Level = tplName.contains("三级");

        // 查询已记录的操作流水日志，统计已通过的中间阶段数
        List<TicketOperationLog> logs = null;
        try {
            logs = ticketOperationLogMapper.selectList(
                    new QueryWrapper<TicketOperationLog>().eq("ticket_id", ticket.getId()).orderByAsc("id")
            );
        } catch (Exception ignored) {}

        int stageApprovedCount = 0;
        String stage1Approver = "自动预检";
        String stage2Approver = null;
        String stage3Approver = null;

        if (logs != null) {
            for (TicketOperationLog l : logs) {
                if ("STAGE_APPROVE".equals(l.getOperationType())) {
                    stageApprovedCount++;
                    if (stageApprovedCount == 1) stage2Approver = l.getOperatorName();
                    if (stageApprovedCount == 2) stage3Approver = l.getOperatorName();
                }
            }
        }

        StringBuilder sb = new StringBuilder();

        if (is4Level) {
            // 四级审批流：1.语法智能预检 -> 2.业务开发组长初审 -> 3.核心DBA安全复核 -> 4.运维安全总监终审 -> 5.自动执行
            sb.append("> 1️⃣【SQL语法与安全智能预检】➔ ✅ 已通过 (系统网关)\n");

            if (stageApprovedCount >= 1) {
                sb.append("> 2️⃣【业务开发组长初审】➔ ✅ 已通过 (").append(stage2Approver != null ? stage2Approver : "开发组长").append(")\n");
            } else if ("AUDITING".equalsIgnoreCase(status) || "SUBMITTED".equalsIgnoreCase(status)) {
                sb.append("> 2️⃣【业务开发组长初审】➔ ⏳ **待审批 (当前节点)**\n");
            } else {
                sb.append("> 2️⃣【业务开发组长初审】➔ ⏱️ 等待中\n");
            }

            if (stageApprovedCount >= 2) {
                sb.append("> 3️⃣【核心DBA安全复核】➔ ✅ 已通过 (").append(stage3Approver != null ? stage3Approver : "核心DBA").append(")\n");
            } else if (stageApprovedCount == 1 && ("AUDITING".equalsIgnoreCase(status) || "SUBMITTED".equalsIgnoreCase(status))) {
                sb.append("> 3️⃣【核心DBA安全复核】➔ ⏳ **待审批 (当前节点)**\n");
            } else {
                sb.append("> 3️⃣【核心DBA安全复核】➔ ⏱️ 等待中\n");
            }

            if ("APPROVED".equalsIgnoreCase(status) || "EXECUTED".equalsIgnoreCase(status) || "WAITING_EXECUTION".equalsIgnoreCase(status)) {
                sb.append("> 4️⃣【运维安全总监终审】➔ ✅ 已终审通过\n");
                sb.append("> 5️⃣【SQL 变更流式执行】➔ ").append("EXECUTED".equalsIgnoreCase(status) ? "✅ 执行完成" : "🚀 执行调度中").append("\n");
            } else if (stageApprovedCount >= 2 && ("AUDITING".equalsIgnoreCase(status) || "SUBMITTED".equalsIgnoreCase(status))) {
                sb.append("> 4️⃣【运维安全总监终审】➔ ⏳ **待终审 (当前节点)**\n");
                sb.append("> 5️⃣【SQL 变更流式执行】➔ ⏱️ 等待中\n");
            } else {
                sb.append("> 4️⃣【运维安全总监终审】➔ ⏱️ 等待中\n");
                sb.append("> 5️⃣【SQL 变更流式执行】➔ ⏱️ 等待中\n");
            }
        } else if (is3Level) {
            // 三级审批流：1.开发组长初审 -> 2.核心DBA技术复审 -> 3.系统管理员终审 -> 4.自动执行
            if (stageApprovedCount >= 1) {
                sb.append("> 1️⃣【开发组长初审】➔ ✅ 已通过 (").append(stage2Approver != null ? stage2Approver : "开发组长").append(")\n");
            } else if ("AUDITING".equalsIgnoreCase(status) || "SUBMITTED".equalsIgnoreCase(status)) {
                sb.append("> 1️⃣【开发组长初审】➔ ⏳ **待审批 (当前节点)**\n");
            } else {
                sb.append("> 1️⃣【开发组长初审】➔ ⏱️ 等待中\n");
            }

            if (stageApprovedCount >= 2) {
                sb.append("> 2️⃣【核心DBA技术复审】➔ ✅ 已通过\n");
            } else if (stageApprovedCount == 1 && ("AUDITING".equalsIgnoreCase(status) || "SUBMITTED".equalsIgnoreCase(status))) {
                sb.append("> 2️⃣【核心DBA技术复审】➔ ⏳ **待复审 (当前节点)**\n");
            } else {
                sb.append("> 2️⃣【核心DBA技术复审】➔ ⏱️ 等待中\n");
            }

            if ("APPROVED".equalsIgnoreCase(status) || "EXECUTED".equalsIgnoreCase(status) || "WAITING_EXECUTION".equalsIgnoreCase(status)) {
                sb.append("> 3️⃣【系统管理员终审】➔ ✅ 已通过\n");
                sb.append("> 4️⃣【SQL 变更执行】➔ ").append("EXECUTED".equalsIgnoreCase(status) ? "✅ 执行完成" : "🚀 执行中").append("\n");
            } else if (stageApprovedCount >= 2 && ("AUDITING".equalsIgnoreCase(status) || "SUBMITTED".equalsIgnoreCase(status))) {
                sb.append("> 3️⃣【系统管理员终审】➔ ⏳ **待终审 (当前节点)**\n");
                sb.append("> 4️⃣【SQL 变更执行】➔ ⏱️ 等待中\n");
            } else {
                sb.append("> 3️⃣【系统管理员终审】➔ ⏱️ 等待中\n");
                sb.append("> 4️⃣【SQL 变更执行】➔ ⏱️ 等待中\n");
            }
        } else {
            // 标准/双人审批流：1.业务组长初审 -> 2.DBA复核终审 -> 3.变更执行
            if ("APPROVED".equalsIgnoreCase(status) || "EXECUTED".equalsIgnoreCase(status) || "WAITING_EXECUTION".equalsIgnoreCase(status)) {
                sb.append("> 1️⃣【业务技术初审】➔ ✅ 已通过\n");
                sb.append("> 2️⃣【DBA 安全复核】➔ ✅ 已通过\n");
                sb.append("> 3️⃣【SQL 变更执行】➔ ").append("EXECUTED".equalsIgnoreCase(status) ? "✅ 执行完成" : "🚀 执行调度中").append("\n");
            } else {
                sb.append("> 1️⃣【业务技术初审】➔ ⏳ **待审批 (当前节点)**\n");
                sb.append("> 2️⃣【DBA 安全复核】➔ ⏱️ 等待中\n");
                sb.append("> 3️⃣【SQL 变更执行】➔ ⏱️ 等待中\n");
            }
        }

        return sb.toString().trim();
    }

    /**
     * 解析工单创建人/申请人的 ERP 账号
     */
    public String resolveApplicantErp(SqlTicket ticket) {
        if (ticket == null) return null;
        String identifier = ticket.getApplicantIdCard();
        if (!StringUtils.hasText(identifier)) identifier = ticket.getApplicantName();
        return findErpByUserIdentifier(identifier);
    }

    /**
     * 解析当前节点的所有待审批人 ERP 列表
     */
    public List<String> resolvePendingApproverErps(SqlTicket ticket) {
        List<String> list = new ArrayList<>();
        if (ticket == null) return list;

        String tplName = ticket.getWorkflowTemplateName() != null ? ticket.getWorkflowTemplateName() : "";
        boolean is4Level = tplName.contains("四级");
        boolean is3Level = tplName.contains("三级");

        int stageApprovedCount = 0;
        try {
            List<TicketOperationLog> logs = ticketOperationLogMapper.selectList(
                    new QueryWrapper<TicketOperationLog>().eq("ticket_id", ticket.getId()).orderByAsc("id")
            );
            if (logs != null) {
                for (TicketOperationLog l : logs) {
                    if ("STAGE_APPROVE".equals(l.getOperationType())) stageApprovedCount++;
                }
            }
        } catch (Exception ignored) {}

        // 根据当前所处的阶段匹配对应角色的审批人
        String targetRole = "DEV_LEAD";
        if (is4Level) {
            if (stageApprovedCount == 0) targetRole = "DEV_LEAD";
            else if (stageApprovedCount == 1) targetRole = "DBA";
            else targetRole = "ADMIN";
        } else if (is3Level) {
            if (stageApprovedCount == 0) targetRole = "DEV_LEAD";
            else if (stageApprovedCount == 1) targetRole = "DBA";
            else targetRole = "ADMIN";
        }

        try {
            List<SysUser> users = sysUserMapper.selectList(new LambdaQueryWrapper<SysUser>()
                    .eq(SysUser::getRole, targetRole)
                    .or().eq(SysUser::getRole, "ADMIN")
            );
            if (users != null) {
                for (SysUser u : users) {
                    String erp = StringUtils.hasText(u.getWorkWechat()) ? u.getWorkWechat() :
                            (StringUtils.hasText(u.getJobNo()) ? u.getJobNo() : u.getUsername());
                    if (StringUtils.hasText(erp) && !list.contains(erp)) {
                        list.add(erp);
                    }
                }
            }
        } catch (Exception e) {
            log.warn("[消息推送] 查找待审批人失败: {}", e.getMessage());
        }

        return list;
    }

    /**
     * 核心企微推送方法（参考 ExternalDeptMessageService WebService WSDL 规范）
     *
     * @param erp 员工 ERP 账号 / 工号 (如 zhangsan, 01088234，严禁传中文名或身份证号)
     * @param msg Markdown 消息文本
     * @return 推送结果响应
     */
    public PushResult sendMessage(String erp, String msg) {
        PushResult result = new PushResult();
        result.setErp(erp);

        NotificationConfigDTO config = notificationConfigService.getRawConfig();
        NotificationConfigDTO.WechatConfig wCfg = config != null ? config.getWechat() : null;

        if (wCfg != null && !Boolean.TRUE.equals(wCfg.getEnabled())) {
            result.setSuccess(false);
            result.setMessage("企业微信通知通道已在后台关闭");
            return result;
        }

        // 校验 ERP 格式，防止误传中文名或 18 位身份证
        if (!StringUtils.hasText(erp)) {
            result.setSuccess(false);
            result.setMessage("ERP 账号不能为空");
            return result;
        }

        if (erp.length() == 18 && erp.matches("\\d{17}[\\dXx]")) {
            log.warn("[消息推送] 检测到传入了 18 位身份证号 [{}]，尝试映射为其 ERP 工号账号", erp);
            String mappedErp = mapIdCardToErp(erp);
            if (StringUtils.hasText(mappedErp)) {
                erp = mappedErp;
                result.setErp(mappedErp);
            }
        }

        if (containsChinese(erp)) {
            log.warn("[消息推送] 检测到传入了中文姓名 [{}]，尝试查找其系统 ERP 账号", erp);
            String mappedErp = mapRealNameToErp(erp);
            if (StringUtils.hasText(mappedErp)) {
                erp = mappedErp;
                result.setErp(mappedErp);
            } else {
                result.setSuccess(false);
                result.setMessage("ERP 参数必须是员工英文工号/ERP账号/企微UserID，不能是中文姓名 [" + erp + "]");
                return result;
            }
        }

        String wsdlEndpoint = (wCfg != null && StringUtils.hasText(wCfg.getWsdlEndpoint()))
                ? wCfg.getWsdlEndpoint()
                : "http://9.0.17.52:8083/wechat-wbs/services/ExternalDeptMessageService?wsdl";
        String sysId = (wCfg != null && StringUtils.hasText(wCfg.getSysId())) ? wCfg.getSysId() : "WMDB_SYSTEM";
        String sysIdPass = (wCfg != null && StringUtils.hasText(wCfg.getSysIdPass())) ? wCfg.getSysIdPass() : "wmdb_pass_123";
        String sysFlag = (wCfg != null && StringUtils.hasText(wCfg.getSysFlag())) ? wCfg.getSysFlag() : "1";

        String sysMessageId = UUID.randomUUID().toString().replace("-", "");
        String serviceUrl = wsdlEndpoint.replace("?wsdl", "").replace("?WSDL", "");

        // 构造标准 SOAP 1.1 / 1.2 XML 请求载荷
        String soapXml = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n" +
                "<soapenv:Envelope xmlns:soapenv=\"http://schemas.xmlsoap.org/soap/envelope/\" xmlns:ser=\"http://service.external.wechat.com/\">\n" +
                "  <soapenv:Header/>\n" +
                "  <soapenv:Body>\n" +
                "    <ser:sendMarkdownMsg>\n" +
                "      <arg0>\n" +
                "        <sys_id>" + escapeXml(sysId) + "</sys_id>\n" +
                "        <sys_id_pass>" + escapeXml(sysIdPass) + "</sys_id_pass>\n" +
                "        <sys_flag>" + escapeXml(sysFlag) + "</sys_flag>\n" +
                "        <msgType>markdown</msgType>\n" +
                "        <userType>2</userType>\n" +
                "        <content><![CDATA[" + msg + "]]></content>\n" +
                "        <sys_message_id>" + sysMessageId + "</sys_message_id>\n" +
                "        <userNum>" + escapeXml(erp.trim()) + "</userNum>\n" +
                "      </arg0>\n" +
                "    </ser:sendMarkdownMsg>\n" +
                "  </soapenv:Body>\n" +
                "</soapenv:Envelope>";

        log.info("[消息推送] 发起企微 WebService 推送: ERP={}, Endpoint={}", erp, serviceUrl);

        HttpURLConnection conn = null;
        try {
            URL url = new URL(serviceUrl);
            conn = (HttpURLConnection) url.openConnection();
            conn.setRequestMethod("POST");
            conn.setDoOutput(true);
            conn.setDoInput(true);
            conn.setConnectTimeout(5000);
            conn.setReadTimeout(8000);
            conn.setRequestProperty("Content-Type", "text/xml; charset=UTF-8");
            conn.setRequestProperty("SOAPAction", "\"\"");

            try (OutputStream os = conn.getOutputStream()) {
                os.write(soapXml.getBytes(StandardCharsets.UTF_8));
                os.flush();
            }

            int responseCode = conn.getResponseCode();
            InputStream is = (responseCode >= 200 && responseCode < 300) ? conn.getInputStream() : conn.getErrorStream();
            StringBuilder respSb = new StringBuilder();
            if (is != null) {
                try (BufferedReader reader = new BufferedReader(new InputStreamReader(is, StandardCharsets.UTF_8))) {
                    String line;
                    while ((line = reader.readLine()) != null) {
                        respSb.append(line);
                    }
                }
            }

            String responseText = respSb.toString();
            log.info("[消息推送] WebService 响应码: {}, 结果: {}", responseCode, responseText);

            if (responseCode == 200) {
                result.setSuccess(true);
                result.setMessage("推送成功");
                result.setRawResponse(responseText);
            } else {
                result.setSuccess(false);
                result.setErrorCode(String.valueOf(responseCode));
                if (responseCode == 500) {
                    result.setMessage("服务端返回 500 错误: 请检查 ERP 账号 [" + erp + "] 是否在通讯录中有效存在，以及 sys_id 认证凭据是否正确。详情: " + responseText);
                } else {
                    result.setMessage("HTTP " + responseCode + " 推送失败: " + responseText);
                }
            }
        } catch (Exception e) {
            log.error("[消息推送] 调用 WebService 异常: {}", e.getMessage(), e);
            result.setSuccess(false);
            result.setMessage("调用失败: " + e.getMessage());
        } finally {
            if (conn != null) {
                conn.disconnect();
            }
        }

        return result;
    }

    /**
     * 解析责任人电话
     */
    private String resolveRecipientPhone(SqlTicket ticket) {
        try {
            String identifier = ticket.getApplicantIdCard();
            if (!StringUtils.hasText(identifier)) identifier = ticket.getApplicantName();
            if (StringUtils.hasText(identifier)) {
                SysUser user = sysUserMapper.selectOne(new LambdaQueryWrapper<SysUser>()
                        .eq(SysUser::getIdCard, identifier)
                        .or().eq(SysUser::getUsername, identifier)
                        .or().eq(SysUser::getRealName, identifier));
                if (user != null && StringUtils.hasText(user.getPhone())) {
                    return user.getPhone();
                }
            }
        } catch (Exception ignored) {}
        return null;
    }

    public String findErpByUserIdentifier(String identifier) {
        if (!StringUtils.hasText(identifier)) return null;

        try {
            LambdaQueryWrapper<SysUser> query = new LambdaQueryWrapper<SysUser>()
                    .eq(SysUser::getIdCard, identifier)
                    .or().eq(SysUser::getUsername, identifier)
                    .or().eq(SysUser::getRealName, identifier)
                    .or().eq(SysUser::getJobNo, identifier)
                    .or().eq(SysUser::getWorkWechat, identifier);

            SysUser user = sysUserMapper.selectOne(query);
            if (user != null) {
                if (StringUtils.hasText(user.getWorkWechat())) {
                    return user.getWorkWechat();
                }
                if (StringUtils.hasText(user.getJobNo())) {
                    return user.getJobNo();
                }
                if (StringUtils.hasText(user.getUsername())) {
                    return user.getUsername();
                }
            }
        } catch (Exception e) {
            log.warn("[消息推送] 检索用户 ERP 账号失败: {}", e.getMessage());
        }

        return identifier;
    }

    private String mapIdCardToErp(String idCard) {
        try {
            SysUser user = sysUserMapper.selectOne(new LambdaQueryWrapper<SysUser>().eq(SysUser::getIdCard, idCard));
            if (user != null) {
                return StringUtils.hasText(user.getWorkWechat()) ? user.getWorkWechat() :
                        (StringUtils.hasText(user.getJobNo()) ? user.getJobNo() : user.getUsername());
            }
        } catch (Exception ignored) {}
        return null;
    }

    private String mapRealNameToErp(String realName) {
        try {
            SysUser user = sysUserMapper.selectOne(new LambdaQueryWrapper<SysUser>().eq(SysUser::getRealName, realName));
            if (user != null) {
                return StringUtils.hasText(user.getWorkWechat()) ? user.getWorkWechat() :
                        (StringUtils.hasText(user.getJobNo()) ? user.getJobNo() : user.getUsername());
            }
        } catch (Exception ignored) {}
        return null;
    }

    private boolean containsChinese(String str) {
        if (str == null) return false;
        for (char c : str.toCharArray()) {
            if (Character.UnicodeScript.of(c) == Character.UnicodeScript.HAN) {
                return true;
            }
        }
        return false;
    }

    private String escapeXml(String text) {
        if (text == null) return "";
        return text.replace("&", "&amp;")
                .replace("<", "&lt;")
                .replace(">", "&gt;")
                .replace("\"", "&quot;")
                .replace("'", "&apos;");
    }

    private String formatStatusZh(String status) {
        if (status == null) return "流转中";
        return switch (status.toUpperCase()) {
            case "SUBMITTED", "AUDITING" -> "待审批流转";
            case "APPROVED" -> "审批已通过";
            case "REJECTED" -> "审批已被驳回";
            case "WAITING_EXECUTION" -> "排队等待执行";
            case "EXECUTING" -> "正在执行 SQL 变更";
            case "EXECUTED" -> "SQL 变更执行成功";
            case "FAILED" -> "SQL 变更执行失败";
            case "TERMINATED" -> "工单已被终止";
            case "WITHDRAWN" -> "工单已撤回";
            default -> status;
        };
    }

    private String getStatusColor(String status) {
        if (status == null) return "info";
        return switch (status.toUpperCase()) {
            case "APPROVED", "EXECUTED" -> "info";
            case "AUDITING", "SUBMITTED", "WAITING_EXECUTION" -> "warning";
            case "REJECTED", "FAILED", "TERMINATED" -> "warning";
            default -> "info";
        };
    }

    @Data
    public static class PushResult {
        private String erp;
        private boolean success;
        private String errorCode;
        private String message;
        private String rawResponse;
    }
}
