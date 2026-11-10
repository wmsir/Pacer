package com.wmdb.mapper;

import com.baomidou.mybatisplus.core.mapper.BaseMapper;
import com.wmdb.model.DataMaskingRule;
import org.apache.ibatis.annotations.Mapper;

/**
 * 数据脱敏规则 Mapper
 *
 * @author wm
 */
@Mapper
public interface DataMaskingRuleMapper extends BaseMapper<DataMaskingRule> {
}
