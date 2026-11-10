package com.wmdb.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

/**
 * BPMN 预置流程模板 DTO
 *
 * @author wm
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class BpmnTemplateDTO {

    private String id;
    private String name;
    private String processKey;
    private String category;
    private String description;
    private String nodes;
    private String bpmnXml;
}
