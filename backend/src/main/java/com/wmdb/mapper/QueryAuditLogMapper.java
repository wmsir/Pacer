package com.wmdb.mapper;

import com.baomidou.mybatisplus.core.mapper.BaseMapper;
import com.wmdb.model.QueryAuditLog;
import org.apache.ibatis.annotations.Mapper;

/**
 * 查询审计日志 Mapper
 *
 * @author wm
 */
@Mapper
public interface QueryAuditLogMapper extends BaseMapper<QueryAuditLog> {
}
