package com.wmdb.model;

import com.baomidou.mybatisplus.core.metadata.IPage;
import io.swagger.v3.oas.annotations.media.Schema;
import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

import java.io.Serializable;
import java.util.ArrayList;
import java.util.List;

/**
 * 统一通用分页数据封装传输对象
 *
 * @param <T> 数据记录实体类型
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
@Schema(description = "统一通用分页数据响应实体")
public class PageResultDTO<T> implements Serializable {

    private static final long serialVersionUID = 1L;

    @Schema(description = "当前页数据记录清单")
    @Builder.Default
    private List<T> records = new ArrayList<>();

    @Schema(description = "符合条件的总记录数", example = "100")
    private long total;

    @Schema(description = "当前页码", example = "1")
    private long current;

    @Schema(description = "每页记录条数", example = "10")
    private long size;

    @Schema(description = "总页数", example = "10")
    private long pages;

    /**
     * 从 MyBatis-Plus IPage 快速构造
     */
    public static <T> PageResultDTO<T> from(IPage<T> page) {
        if (page == null) {
            return PageResultDTO.<T>builder()
                    .records(new ArrayList<>())
                    .total(0)
                    .current(1)
                    .size(10)
                    .pages(0)
                    .build();
        }
        return PageResultDTO.<T>builder()
                .records(page.getRecords() != null ? page.getRecords() : new ArrayList<>())
                .total(page.getTotal())
                .current(page.getCurrent())
                .size(page.getSize())
                .pages(page.getPages())
                .build();
    }

    /**
     * 自定义构造
     */
    public static <T> PageResultDTO<T> of(List<T> records, long total, long current, long size) {
        long pages = size > 0 ? (total + size - 1) / size : 0;
        return PageResultDTO.<T>builder()
                .records(records != null ? records : new ArrayList<>())
                .total(total)
                .current(current)
                .size(size)
                .pages(pages)
                .build();
    }
}
