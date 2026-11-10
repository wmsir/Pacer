package com.wmdb.service;

import com.baomidou.mybatisplus.core.conditions.query.QueryWrapper;
import com.wmdb.mapper.SysUserMapper;
import com.wmdb.model.SysUser;
import org.springframework.security.crypto.password.PasswordEncoder;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

/**
 * 外部组织架构同步对接服务
 * <p>
 * 接收外部企业级系统（如 OA/HR/LDAP）的用户推送，过滤并同步至本地 RBAC 体系。
 * 动态将外部的部门/岗位标签映射为系统内置的 role，用以驱动 Flowable 动态审批连线。
 * </p>
 *
 * @author wm
 */
@Service
public class ExternalSyncService {

    private final SysUserMapper sysUserMapper;
    private final PasswordEncoder passwordEncoder;

    public ExternalSyncService(SysUserMapper sysUserMapper, PasswordEncoder passwordEncoder) {
        this.sysUserMapper = sysUserMapper;
        this.passwordEncoder = passwordEncoder;
    }

    /**
     * 同步外部用户至本地库
     *
     * @param realName 外部系统提供的真实姓名
     * @param idCard 外部系统提供的身份证号码（唯一标识）
     * @param externalDepartment 外部系统的部门或岗位特征
     */
    @Transactional(rollbackFor = Exception.class)
    public void syncUser(String realName, String idCard, String externalDepartment, String tenantId) {
        // 动态角色映射逻辑 (Dynamic Role Mapping)
        String mappedRole = mapExternalDepartmentToRole(externalDepartment);

        SysUser existingUser = sysUserMapper.selectOne(new QueryWrapper<SysUser>().eq("id_card", idCard));
        if (existingUser != null) {
            // Update existing
            existingUser.setRealName(realName);
            existingUser.setRole(mappedRole);
            if (tenantId != null && !tenantId.isEmpty()) {
                existingUser.setTenantId(tenantId);
            }
            sysUserMapper.updateById(existingUser);
        } else {
            // Insert new user
            SysUser newUser = new SysUser();
            newUser.setRealName(realName);
            newUser.setIdCard(idCard);
            newUser.setRole(mappedRole);
            if (tenantId != null && !tenantId.isEmpty()) {
                newUser.setTenantId(tenantId);
            }

            // Generate a default temporary password for new synced users, or handle SSO exclusively
            String defaultPwd = idCard.substring(idCard.length() - 6); // default to last 6 digits of ID
            newUser.setPasswordCipher(passwordEncoder.encode(defaultPwd));

            sysUserMapper.insert(newUser);
        }
    }

    /**
     * 将外部部门或岗位特征转换为本地系统 role
     */
    private String mapExternalDepartmentToRole(String externalDepartment) {
        if (externalDepartment == null) return "DEV";

        String lowerDept = externalDepartment.toLowerCase();
        if (lowerDept.contains("dba") || lowerDept.contains("database")) {
            return "DBA";
        } else if (lowerDept.contains("admin") || lowerDept.contains("security")) {
            return "ADMIN";
        } else {
            return "DEV";
        }
    }
}
