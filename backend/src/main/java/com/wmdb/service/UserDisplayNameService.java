package com.wmdb.service;

import com.baomidou.mybatisplus.core.conditions.query.QueryWrapper;
import com.wmdb.mapper.SysUserMapper;
import com.wmdb.model.SysUser;
import com.wmdb.model.SysUserDTO;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Service;

import java.util.*;
import java.util.stream.Collectors;

/**
 * 用户显示名称与同名同姓智能消歧服务
 * <p>
 * 当系统中存在同名同姓的用户时，系统自动识别并附加【身份证后6位】或【手机尾号】进行精准消歧展示，
 * 确保审批流节点、工单责任人、用户列表中权责清晰可辨。
 * </p>
 *
 * @author wm
 */
@Slf4j
@Service
@RequiredArgsConstructor
public class UserDisplayNameService {

    private final SysUserMapper sysUserMapper;

    /**
     * 计算单个用户的显示名称（带智能同名消歧）
     *
     * @param user 用户实体
     * @return 格式化后的消歧名称（如 "张三" 或 "张三 (尾号: 011234)"）
     */
    public String getDisplayName(SysUser user) {
        if (user == null) {
            return "";
        }
        String realName = user.getRealName();
        if (realName == null || realName.trim().isEmpty()) {
            return user.getUsername() != null ? user.getUsername() : (user.getIdCard() != null ? user.getIdCard() : "未知用户");
        }
        realName = realName.trim();

        Long count = sysUserMapper.selectCount(new QueryWrapper<SysUser>().eq("real_name", realName));
        if (count == null || count <= 1) {
            return realName;
        }

        return formatDisambiguationSuffix(realName, user.getIdCard(), user.getPhone(), user.getUsername());
    }

    /**
     * 批量为用户列表注入消歧显示名称
     *
     * @param userList 用户实体列表
     * @return 包含 displayName 的 DTO 列表
     */
    public List<SysUserDTO> formatUserList(List<SysUser> userList) {
        if (userList == null || userList.isEmpty()) {
            return Collections.emptyList();
        }

        // 统计全库或本批次中的重名频次
        Map<String, Long> sameNameMap = new HashMap<>();
        for (SysUser u : userList) {
            if (u.getRealName() != null && !u.getRealName().trim().isEmpty()) {
                String rn = u.getRealName().trim();
                if (!sameNameMap.containsKey(rn)) {
                    Long dbCount = sysUserMapper.selectCount(new QueryWrapper<SysUser>().eq("real_name", rn));
                    sameNameMap.put(rn, dbCount != null ? dbCount : 1L);
                }
            }
        }

        return userList.stream().map(u -> {
            String realName = u.getRealName() != null ? u.getRealName().trim() : "";
            String displayName = realName;
            Long freq = sameNameMap.get(realName);

            if (freq != null && freq > 1) {
                displayName = formatDisambiguationSuffix(realName, u.getIdCard(), u.getPhone(), u.getUsername());
            } else if (realName.isEmpty()) {
                displayName = u.getUsername() != null ? u.getUsername() : u.getIdCard();
            }
            List<String> rgs = parseResourceGroups(u.getResourceGroup());
            List<String> roleList = parseRoles(u.getRole());
            List<String> customPerms = parsePermissions(u.getPermissions());

            return SysUserDTO.builder()
                    .id(u.getId())
                    .tenantId(u.getTenantId())
                    .username(u.getUsername())
                    .realName(u.getRealName())
                    .displayName(displayName)
                    .idCard(u.getIdCard())
                    .phone(u.getPhone())
                    .email(u.getEmail())
                    .role(String.join(", ", roleList))
                    .roles(roleList)
                    .resourceGroup(String.join(", ", rgs))
                    .resourceGroups(rgs)
                    .permissions(customPerms)
                    .customPermissions(customPerms)
                    .wechat(u.getWechat())
                    .workWechat(u.getWorkWechat())
                    .dingtalk(u.getDingtalk())
                    .feishu(u.getFeishu())
                    .department(u.getDepartment())
                    .jobNo(u.getJobNo())
                    .notificationPrefs(u.getNotificationPrefs())
                    .ticketDataScope(u.getTicketDataScope())
                    .status(u.getStatus())
                    .build();
        }).collect(Collectors.toList());
    }

    public List<String> parseRoles(String raw) {
        List<String> list = new ArrayList<>();
        if (raw == null || raw.trim().isEmpty()) {
            list.add("DEV");
            return list;
        }
        String cleaned = raw.trim();
        if (cleaned.startsWith("[") && cleaned.endsWith("]")) {
            try {
                com.fasterxml.jackson.databind.ObjectMapper mapper = new com.fasterxml.jackson.databind.ObjectMapper();
                list = mapper.readValue(cleaned, new com.fasterxml.jackson.core.type.TypeReference<List<String>>() {});
                return list.isEmpty() ? List.of("DEV") : list;
            } catch (Exception ignored) {
                cleaned = cleaned.substring(1, cleaned.length() - 1);
            }
        }
        String[] parts = cleaned.split("[,，/]");
        for (String p : parts) {
            String item = p.replace("\"", "").replace("'", "").trim();
            if (!item.isEmpty() && !list.contains(item)) {
                list.add(item);
            }
        }
        return list.isEmpty() ? List.of("DEV") : list;
    }

    public List<String> parsePermissions(String raw) {
        List<String> list = new ArrayList<>();
        if (raw == null || raw.trim().isEmpty()) {
            return list;
        }
        String cleaned = raw.trim();
        if (cleaned.startsWith("[") && cleaned.endsWith("]")) {
            try {
                com.fasterxml.jackson.databind.ObjectMapper mapper = new com.fasterxml.jackson.databind.ObjectMapper();
                return mapper.readValue(cleaned, new com.fasterxml.jackson.core.type.TypeReference<List<String>>() {});
            } catch (Exception ignored) {
                cleaned = cleaned.substring(1, cleaned.length() - 1);
            }
        }
        String[] parts = cleaned.split("[,，]");
        for (String p : parts) {
            String item = p.replace("\"", "").replace("'", "").trim();
            if (!item.isEmpty() && !list.contains(item)) {
                list.add(item);
            }
        }
        return list;
    }

    public List<String> parseResourceGroups(String raw) {
        List<String> list = new ArrayList<>();
        if (raw == null || raw.trim().isEmpty()) {
            return list;
        }
        String cleaned = raw.trim();
        if (cleaned.startsWith("[") && cleaned.endsWith("]")) {
            cleaned = cleaned.substring(1, cleaned.length() - 1);
        }
        String[] parts = cleaned.split("[,，]");
        for (String p : parts) {
            String item = p.replace("\"", "").replace("'", "").trim();
            if (!item.isEmpty() && !list.contains(item)) {
                list.add(item);
            }
        }
        return list;
    }

    /**
     * 按照 身份证尾号 > 手机尾号 > 账号名 的优先级生成消歧后缀
     */
    private String formatDisambiguationSuffix(String realName, String idCard, String phone, String username) {
        if (idCard != null && idCard.trim().length() >= 6) {
            String trimmed = idCard.trim();
            String tail = trimmed.substring(trimmed.length() - 6);
            return realName + " (尾号: " + tail + ")";
        }
        if (phone != null && phone.trim().length() >= 6) {
            String trimmed = phone.trim();
            String tail = trimmed.substring(trimmed.length() - 6);
            return realName + " (手机尾号: " + tail + ")";
        }
        if (username != null && !username.trim().isEmpty()) {
            return realName + " (@" + username.trim() + ")";
        }
        return realName;
    }
}
