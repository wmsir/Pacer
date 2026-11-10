package com.wmdb.service;

import com.wmdb.security.SmUtils;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Service;

import java.util.UUID;

/**
 * 区块链存证服务 (Blockchain Evidence Service)
 * <p>
 * 模拟对接长安链 (ChainMaker) 等国产区块链底层平台，对敏感 SQL 执行操作进行 SM3 哈希存证。
 * 确保证据不可篡改，符合司法存证与审计要求。
 * </p>
 *
 * @author wm
 */
@Slf4j
@Service
public class BlockchainService {

    /**
     * 将关键操作记录上链存证
     *
     * @param ticketId 工单 ID
     * @param sqlText 实际执行的 SQL
     * @param operatorIdCard 执行人身份证号
     * @return 存证流水号 (Transaction ID)
     */
    public String preserveEvidence(Long ticketId, String sqlText, String operatorIdCard) {
        String payload = String.format("Ticket:%d|User:%s|SQL:%s|Time:%d", ticketId, operatorIdCard, sqlText, System.currentTimeMillis());

        // 使用国密 SM3 生成防篡改 Hash
        String hash = SmUtils.sm3Hash(payload);

        // 模拟上链过程
        String txId = "TX-" + UUID.randomUUID().toString().replace("-", "").toUpperCase();

        log.info("==> [区块链存证] 上链成功! TicketId: {}, Hash: {}, TxID: {}", ticketId, hash, txId);

        return txId;
    }
}
