package com.wmdb.service;

import com.baomidou.mybatisplus.core.conditions.query.QueryWrapper;
import com.wmdb.exception.BusinessException;
import com.wmdb.mapper.SysUserMapper;
import com.wmdb.model.SysUser;
import com.wmdb.model.SysUserDTO;
import com.wmdb.security.SmUtils;
import jakarta.annotation.PostConstruct;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Service;

import javax.sql.DataSource;
import java.sql.Connection;
import java.sql.DatabaseMetaData;
import java.sql.ResultSet;
import java.sql.Statement;
import java.util.HashSet;
import java.util.List;
import java.util.Set;

/**
 * 系统用户业务服务
 *
 * @author wm
 */
@Slf4j
@Service
@RequiredArgsConstructor
public class UserService {

    private final SysUserMapper sysUserMapper;
    private final DataSource dataSource;
    private final UserDisplayNameService userDisplayNameService;

    @PostConstruct
    public void initUserColumns() {
        try (Connection conn = dataSource.getConnection()) {
            DatabaseMetaData md = conn.getMetaData();
            String catalog = conn.getCatalog();
            String schema = conn.getSchema();

            Set<String> existingCols = new HashSet<>();
            try (ResultSet rs = md.getColumns(catalog, schema, "sys_user", "%")) {
                while (rs.next()) {
                    existingCols.add(rs.getString("COLUMN_NAME").toLowerCase());
                }
            }

            try (Statement stmt = conn.createStatement()) {
                if (!existingCols.contains("resource_group")) {
                    stmt.execute("ALTER TABLE sys_user ADD COLUMN resource_group VARCHAR(500);");
                    log.info("Added resource_group column to sys_user.");
                } else {
                    try {
                        stmt.execute("ALTER TABLE sys_user MODIFY COLUMN resource_group VARCHAR(500);");
                    } catch (Exception ignored) {}
                }
                try {
                    stmt.execute("ALTER TABLE sys_user MODIFY COLUMN role VARCHAR(1000);");
                    log.info("Modified role column to VARCHAR(1000) on sys_user.");
                } catch (Exception ignored) {}
                if (!existingCols.contains("email")) {
                    stmt.execute("ALTER TABLE sys_user ADD COLUMN email VARCHAR(100);");
                    log.info("Added email column to sys_user.");
                }
                if (!existingCols.contains("permissions")) {
                    stmt.execute("ALTER TABLE sys_user ADD COLUMN permissions TEXT;");
                    log.info("Added permissions column to sys_user.");
                }
                if (!existingCols.contains("wechat")) {
                    stmt.execute("ALTER TABLE sys_user ADD COLUMN wechat VARCHAR(100);");
                    log.info("Added wechat column to sys_user.");
                }
                if (!existingCols.contains("work_wechat")) {
                    stmt.execute("ALTER TABLE sys_user ADD COLUMN work_wechat VARCHAR(100);");
                    log.info("Added work_wechat column to sys_user.");
                }
                if (!existingCols.contains("dingtalk")) {
                    stmt.execute("ALTER TABLE sys_user ADD COLUMN dingtalk VARCHAR(100);");
                    log.info("Added dingtalk column to sys_user.");
                }
                if (!existingCols.contains("feishu")) {
                    stmt.execute("ALTER TABLE sys_user ADD COLUMN feishu VARCHAR(100);");
                    log.info("Added feishu column to sys_user.");
                }
                if (!existingCols.contains("department")) {
                    stmt.execute("ALTER TABLE sys_user ADD COLUMN department VARCHAR(100);");
                    log.info("Added department column to sys_user.");
                }
                if (!existingCols.contains("job_no")) {
                    stmt.execute("ALTER TABLE sys_user ADD COLUMN job_no VARCHAR(100);");
                    log.info("Added job_no column to sys_user.");
                }
                if (!existingCols.contains("notification_prefs")) {
                    stmt.execute("ALTER TABLE sys_user ADD COLUMN notification_prefs VARCHAR(1000);");
                    log.info("Added notification_prefs column to sys_user.");
                }
                if (!existingCols.contains("ticket_data_scope")) {
                    stmt.execute("ALTER TABLE sys_user ADD COLUMN ticket_data_scope VARCHAR(50);");
                    log.info("Added ticket_data_scope column to sys_user.");
                }
            }
        } catch (Exception e) {
            log.warn("Init sys_user columns exception: {}", e.getMessage());
        }
    }

    public SysUser getUserByIdentifier(String identifier) {
        if (identifier == null || identifier.trim().isEmpty()) {
            return null;
        }
        final String search = identifier.trim();
        return sysUserMapper.selectOne(new QueryWrapper<SysUser>()
                .eq("username", search)
                .or().eq("id_card", search)
                .or().eq("phone", search)
                .last("LIMIT 1"));
    }

    public SysUserDTO getCurrentUserProfile(String identifier) {
        SysUser user = getUserByIdentifier(identifier);
        if (user == null) {
            throw new BusinessException("A0404", "用户不存在或登录已失效");
        }
        List<SysUserDTO> dtos = userDisplayNameService.formatUserList(List.of(user));
        return dtos.isEmpty() ? null : dtos.get(0);
    }

    public void updateCurrentUserProfile(String identifier, com.wmdb.model.UserProfileUpdateDTO req) {
        SysUser user = getUserByIdentifier(identifier);
        if (user == null) {
            throw new BusinessException("A0404", "用户不存在或登录已失效");
        }
        if (req.getRealName() != null && !req.getRealName().trim().isEmpty()) {
            user.setRealName(req.getRealName().trim());
        }
        if (req.getPhone() != null) {
            user.setPhone(req.getPhone().trim());
        }
        if (req.getEmail() != null) {
            user.setEmail(req.getEmail().trim());
        }
        if (req.getWechat() != null) {
            user.setWechat(req.getWechat().trim());
        }
        if (req.getWorkWechat() != null) {
            user.setWorkWechat(req.getWorkWechat().trim());
        }
        if (req.getDingtalk() != null) {
            user.setDingtalk(req.getDingtalk().trim());
        }
        if (req.getFeishu() != null) {
            user.setFeishu(req.getFeishu().trim());
        }
        if (req.getDepartment() != null) {
            user.setDepartment(req.getDepartment().trim());
        }
        if (req.getJobNo() != null) {
            user.setJobNo(req.getJobNo().trim());
        }
        if (req.getNotificationPrefs() != null) {
            user.setNotificationPrefs(req.getNotificationPrefs().trim());
        }

        sysUserMapper.updateById(user);
    }

    public void changeCurrentUserPassword(String identifier, com.wmdb.model.ChangePasswordRequestDTO req) {
        if (req == null || req.getNewPassword() == null || req.getNewPassword().trim().isEmpty()) {
            throw new BusinessException("A0400", "请输入新密码");
        }
        if (!req.getNewPassword().equals(req.getConfirmPassword())) {
            throw new BusinessException("A0400", "两次输入的新密码不一致");
        }
        SysUser user = getUserByIdentifier(identifier);
        if (user == null) {
            throw new BusinessException("A0404", "用户不存在或登录已失效");
        }

        // 解密旧密码与新密码（兼容 SM2 密文与明文）
        String oldPwd = req.getOldPassword();
        try {
            oldPwd = SmUtils.sm2Decrypt(oldPwd);
        } catch (Exception ignored) {}

        String newPwd = req.getNewPassword();
        try {
            newPwd = SmUtils.sm2Decrypt(newPwd);
        } catch (Exception ignored) {}

        if (newPwd.length() < 6) {
            throw new BusinessException("A0400", "新密码长度不能少于 6 位");
        }

        // 校验旧密码（严格采用国密 SM3 哈希校验，杜绝硬编码后门）
        boolean matched = false;
        if (user.getPasswordCipher() != null && !user.getPasswordCipher().isEmpty()) {
            matched = SmUtils.sm3Matches(oldPwd, user.getPasswordCipher());
        }

        if (!matched) {
            throw new BusinessException("A0400", "原登录密码校验错误，请重新输入");
        }

        String newHash = SmUtils.sm3Hash(newPwd);
        user.setPasswordCipher(newHash);
        user.setPasswordHash(newHash);
        sysUserMapper.updateById(user);
    }

    public List<SysUserDTO> listUsers(String keyword) {
        QueryWrapper<SysUser> qw = new QueryWrapper<>();
        if (keyword != null && !keyword.trim().isEmpty()) {
            qw.like("username", keyword.trim())
                    .or()
                    .like("real_name", keyword.trim())
                    .or()
                    .like("phone", keyword.trim())
                    .or()
                    .like("id_card", keyword.trim())
                    .or()
                    .like("role", keyword.trim())
                    .or()
                    .like("department", keyword.trim())
                    .or()
                    .like("job_no", keyword.trim())
                    .or()
                    .like("resource_group", keyword.trim());
        }
        qw.orderByDesc("id");
        List<SysUser> users = sysUserMapper.selectList(qw);
        return userDisplayNameService.formatUserList(users);
    }

    public com.wmdb.model.PageResultDTO<SysUserDTO> pageUsers(int page, int size, String keyword, String role, Integer status) {
        QueryWrapper<SysUser> qw = new QueryWrapper<>();
        if (keyword != null && !keyword.trim().isEmpty()) {
            String kw = keyword.trim();
            String roleAlias = mapRoleAlias(kw);
            qw.and(w -> {
                w.like("username", kw)
                        .or().like("real_name", kw)
                        .or().like("phone", kw)
                        .or().like("id_card", kw)
                        .or().like("role", kw)
                        .or().like("department", kw)
                        .or().like("job_no", kw)
                        .or().like("resource_group", kw);
                if (roleAlias != null && !roleAlias.isEmpty()) {
                    w.or().like("role", roleAlias);
                }
            });
        }
        if (role != null && !role.trim().isEmpty()) {
            String r = role.trim();
            String roleAlias = mapRoleAlias(r);
            if (roleAlias != null) {
                qw.and(w -> w.like("role", r).or().like("role", roleAlias));
            } else {
                qw.like("role", r);
            }
        }
        if (status != null) {
            qw.eq("status", status);
        }
        qw.orderByDesc("id");

        com.baomidou.mybatisplus.extension.plugins.pagination.Page<SysUser> mpPage =
                new com.baomidou.mybatisplus.extension.plugins.pagination.Page<>(page > 0 ? page : 1, size > 0 ? size : 10);
        sysUserMapper.selectPage(mpPage, qw);

        List<SysUserDTO> dtos = userDisplayNameService.formatUserList(mpPage.getRecords());
        return com.wmdb.model.PageResultDTO.of(dtos, mpPage.getTotal(), mpPage.getCurrent(), mpPage.getSize());
    }

    private String mapRoleAlias(String kw) {
        if (kw == null) return null;
        String lower = kw.toLowerCase().trim();
        if (lower.contains("研发") || lower.contains("开发人员") || lower.contains("工程师") || lower.contains("研发工程师")) return "DEV";
        if (lower.contains("组长") || lower.contains("主管") || lower.contains("dev_lead")) return "DEV_LEAD";
        if (lower.contains("管理员") || lower.contains("超管") || lower.contains("admin")) return "ADMIN";
        if (lower.contains("dba") || lower.contains("数据库管理员") || lower.contains("架构师")) return "DBA";
        if (lower.contains("审计") || lower.contains("合规") || lower.contains("auditor")) return "AUDITOR";
        if (lower.contains("运维") || lower.contains("ops")) return "OPS";
        if (lower.contains("安全") || lower.contains("cso") || lower.contains("security")) return "SECURITY";
        return null;
    }

    public void saveUser(SysUser user) {
        if (user.getUsername() == null || user.getUsername().trim().isEmpty()) {
            throw new BusinessException("A0400", "用户名不能为空");
        }
        if (user.getTenantId() == null || user.getTenantId().isEmpty()) {
            user.setTenantId("1");
        }
        if (user.getStatus() == null) {
            user.setStatus(1);
        }
        if (user.getRole() == null || user.getRole().isEmpty()) {
            user.setRole("DEV");
        }

        // 密码加密处理
        if (user.getPasswordCipher() != null && !user.getPasswordCipher().trim().isEmpty()) {
            String rawPwd = user.getPasswordCipher().trim();
            try {
                rawPwd = SmUtils.sm2Decrypt(rawPwd);
            } catch (Exception ignored) {}
            String hash = SmUtils.sm3Hash(rawPwd);
            user.setPasswordCipher(hash);
            user.setPasswordHash(hash);
        }

        if (user.getId() == null) {
            // 校验重复
            Long existCount = sysUserMapper.selectCount(new QueryWrapper<SysUser>()
                    .eq("username", user.getUsername())
                    .or()
                    .eq(user.getIdCard() != null && !user.getIdCard().isEmpty(), "id_card", user.getIdCard()));
            if (existCount > 0) {
                throw new BusinessException("A0400", "用户名或身份证号已存在");
            }
            if (user.getPasswordCipher() == null || user.getPasswordCipher().isEmpty()) {
                // 默认初始密码 123456
                String defaultHash = SmUtils.sm3Hash("123456");
                user.setPasswordCipher(defaultHash);
                user.setPasswordHash(defaultHash);
            }
            sysUserMapper.insert(user);
        } else {
            // 编辑更新
            if (user.getPasswordCipher() == null || user.getPasswordCipher().isEmpty()) {
                // 不修改密码
                SysUser old = sysUserMapper.selectById(user.getId());
                if (old != null) {
                    user.setPasswordCipher(old.getPasswordCipher());
                    user.setPasswordHash(old.getPasswordHash());
                }
            }
            sysUserMapper.updateById(user);
        }
    }

    public void deleteUser(Long id) {
        sysUserMapper.deleteById(id);
    }

    public void toggleStatus(Long id) {
        SysUser user = sysUserMapper.selectById(id);
        if (user != null) {
            user.setStatus(user.getStatus() != null && user.getStatus() == 1 ? 0 : 1);
            sysUserMapper.updateById(user);
        }
    }

    public void resetPassword(Long id, String newPassword) {
        SysUser user = sysUserMapper.selectById(id);
        if (user != null) {
            String pwd = (newPassword != null && !newPassword.trim().isEmpty()) ? newPassword.trim() : "123456";
            try {
                pwd = SmUtils.sm2Decrypt(pwd);
            } catch (Exception ignored) {}
            String hash = SmUtils.sm3Hash(pwd);
            user.setPasswordCipher(hash);
            user.setPasswordHash(hash);
            sysUserMapper.updateById(user);
        }
    }
}
