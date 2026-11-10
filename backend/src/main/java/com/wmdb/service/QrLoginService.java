package com.wmdb.service;

import com.baomidou.mybatisplus.core.conditions.query.QueryWrapper;
import com.wmdb.exception.BusinessException;
import com.wmdb.mapper.SysUserMapper;
import com.wmdb.model.QrLoginDTO;
import com.wmdb.model.SysUser;
import com.wmdb.model.SysUserDTO;
import com.wmdb.security.JwtUtils;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Service;

import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;

/**
 * 国内主流 APP 扫码登录业务服务
 * <p>
 * 支持企业微信 (WeCom)、钉钉 (DingTalk)、飞书 (Feishu) 与 统一企业 SSO 扫码集成，
 * 包含二维码生命周期管理（等待扫码 -> 已扫码 -> 确认授权 -> 签发 Token）及开放平台 Webhook 预留。
 * </p>
 *
 * @author wm
 */
@Slf4j
@Service
@RequiredArgsConstructor
public class QrLoginService {

    private final SysUserMapper sysUserMapper;
    private final UserDisplayNameService userDisplayNameService;
    private final JwtUtils jwtUtils;

    // 二维码内存会话缓存
    private static final Map<String, QrSession> QR_CACHE = new ConcurrentHashMap<>();

    private static class QrSession {
        String qrKey;
        String channel;
        String channelName;
        String status; // WAITING, SCANNED, CONFIRMED, EXPIRED
        String statusMsg;
        long createTime;
        long expireTime;
        String token;
        SysUserDTO user;
    }

    /**
     * 生成各平台扫码登录会话
     */
    public QrLoginDTO generateQr(String channel) {
        if (channel == null || channel.trim().isEmpty()) {
            channel = "WECOM";
        }
        channel = channel.toUpperCase().trim();

        String channelName = switch (channel) {
            case "WECOM" -> "企业微信";
            case "DINGTALK" -> "钉钉";
            case "FEISHU" -> "飞书";
            case "SSO" -> "统一单点登录 (SSO)";
            default -> "主流移动办公 APP";
        };

        String qrKey = "QR_" + UUID.randomUUID().toString().replace("-", "");
        long now = System.currentTimeMillis();
        long expireAt = now + 3 * 60 * 1000L; // 3 分钟过期

        QrSession session = new QrSession();
        session.qrKey = qrKey;
        session.channel = channel;
        session.channelName = channelName;
        session.status = "WAITING";
        session.statusMsg = "请使用 " + channelName + " 扫描二维码登录";
        session.createTime = now;
        session.expireTime = expireAt;

        QR_CACHE.put(qrKey, session);

        // 模拟各企业开放平台二维码跳转/Schema 链接
        String qrUrl = switch (channel) {
            case "WECOM" -> "https://open.work.weixin.qq.com/wwopen/sso/qrConnect?appid=ww_demo_enterprise&agentid=1000002&redirect_uri=http%3A%2F%2Fwmdb.local%2Fqr%2Fcallback&state=" + qrKey;
            case "DINGTALK" -> "https://login.dingtalk.com/oauth2/auth?redirect_uri=http%3A%2F%2Fwmdb.local%2Fqr%2Fcallback&response_type=code&client_id=ding_demo_app&scope=openid&state=" + qrKey;
            case "FEISHU" -> "https://passport.feishu.cn/suite/pas/oauth/authorize?app_id=cli_demo_feishu&redirect_uri=http%3A%2F%2Fwmdb.local%2Fqr%2Fcallback&response_type=code&state=" + qrKey;
            default -> "http://wmdb.local/sso/login?client_id=wmdb_sso&state=" + qrKey;
        };

        return QrLoginDTO.builder()
                .qrKey(qrKey)
                .channel(channel)
                .channelName(channelName)
                .qrUrl(qrUrl)
                .qrContent(qrUrl)
                .status("WAITING")
                .statusMsg("请使用 " + channelName + " 扫描二维码登录")
                .expireSeconds(180L)
                .build();
    }

    /**
     * 轮询扫码状态
     */
    public QrLoginDTO checkStatus(String qrKey) {
        if (qrKey == null || !QR_CACHE.containsKey(qrKey)) {
            return QrLoginDTO.builder()
                    .qrKey(qrKey)
                    .status("EXPIRED")
                    .statusMsg("二维码已失效，请点击刷新")
                    .expireSeconds(0L)
                    .build();
        }

        QrSession session = QR_CACHE.get(qrKey);
        long now = System.currentTimeMillis();
        if (now > session.expireTime) {
            session.status = "EXPIRED";
            session.statusMsg = "二维码已过期，请刷新重新获取";
        }

        long remainSeconds = Math.max(0, (session.expireTime - now) / 1000);

        return QrLoginDTO.builder()
                .qrKey(session.qrKey)
                .channel(session.channel)
                .channelName(session.channelName)
                .status(session.status)
                .statusMsg(session.statusMsg)
                .expireSeconds(remainSeconds)
                .token(session.token)
                .user(session.user)
                .build();
    }

    /**
     * 模拟移动端扫码与确认授权（便于开发/测试与演练）
     */
    public QrLoginDTO mockScanAndConfirm(String qrKey, String account) {
        if (qrKey == null || !QR_CACHE.containsKey(qrKey)) {
            throw new BusinessException("A0400", "二维码已过期或不存在");
        }

        QrSession session = QR_CACHE.get(qrKey);
        if (System.currentTimeMillis() > session.expireTime) {
            session.status = "EXPIRED";
            session.statusMsg = "二维码已过期，请刷新";
            throw new BusinessException("A0400", "二维码已过期，请刷新");
        }

        final String searchAccount = (account == null || account.trim().isEmpty()) ? "testadmin1" : account.trim();

        // 查找用户
        List<SysUser> userList = sysUserMapper.selectList(new QueryWrapper<SysUser>()
                .and(w -> w.eq("username", searchAccount)
                        .or().eq("id_card", searchAccount)
                        .or().eq("phone", searchAccount))
                .last("LIMIT 1"));
        SysUser user = (userList != null && !userList.isEmpty()) ? userList.get(0) : null;

        if (user == null) {
            // 如果不存在，获取系统内任一可用用户
            user = sysUserMapper.selectOne(new QueryWrapper<SysUser>().last("LIMIT 1"));
        }

        if (user == null) {
            throw new BusinessException("A0400", "未找到可关联的扫码用户");
        }

        String identifier = user.getIdCard() != null ? user.getIdCard() : user.getUsername();
        String realName = user.getRealName() != null ? user.getRealName() : user.getUsername();
        String token = jwtUtils.generateToken(identifier, realName);

        String displayName = userDisplayNameService.getDisplayName(user);

        SysUserDTO userDTO = SysUserDTO.builder()
                .id(user.getId())
                .tenantId(user.getTenantId())
                .username(user.getUsername())
                .realName(user.getRealName())
                .displayName(displayName)
                .idCard(user.getIdCard())
                .phone(user.getPhone())
                .email(user.getEmail())
                .role(user.getRole())
                .resourceGroup(user.getResourceGroup())
                .status(user.getStatus())
                .build();

        session.status = "CONFIRMED";
        session.statusMsg = session.channelName + " 扫码授权成功，正在进入系统...";
        session.token = token;
        session.user = userDTO;

        log.info("QR Login Confirmed via {}: user={}", session.channel, user.getUsername());

        return checkStatus(qrKey);
    }

    /**
     * 开放平台 OAuth2 回调预留入口 (企业微信/钉钉/飞书)
     */
    public String handleOAuthCallback(String channel, String authCode, String stateQrKey) {
        log.info("Received OAuth Callback from {}: code={}, state={}", channel, authCode, stateQrKey);
        if (stateQrKey != null && QR_CACHE.containsKey(stateQrKey)) {
            // 在实际开放平台对接中，此处调用各平台的 get_user_info_by_code API，
            // 获取员工工号/手机号并匹配 sys_user
            mockScanAndConfirm(stateQrKey, "testadmin1");
            return "扫码授权成功，请返回原浏览器窗口";
        }
        return "无效的扫码会话";
    }
}
