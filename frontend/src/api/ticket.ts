import request from '../utils/request';

/**
 * 获取工单超大附件的安全下载链接
 * <p>
 * 调用后端 API 获取 MinIO 的预签名 URL，实现防盗链下载。
 * 此链接通常具有极短的有效期（如 5 分钟）。
 * </p>
 *
 * @param ticketId 工单 ID
 * @returns 预签名下载 URL
 */
export const downloadTicketAttachment = async (ticketId: string): Promise<string> => {
    try {
        const response: any = await request.get(`/v1/ticket/${ticketId}/download-url`);
        return response.data.url;
    } catch (error) {
        throw new Error('无法获取预签名下载链接');
    }
}
