/**
 * 动态实名水印指令 (可配置显隐与透明度)
 *
 * 用于在页面元素上铺满防删除的斜向半透明水印。
 * 水印内容通过解析 localStorage 中的 JWT Token 获取（格式：真实姓名_身份证后四位）。
 * 支持从 localStorage 读取透明度/显隐配置，并支持全局事件即时热更新。
 */
import type { Directive, DirectiveBinding } from 'vue';
import { jwtDecode } from 'jwt-decode';

/**
 * 解析后的 JWT Token 数据结构
 */
interface DecodedToken {
  sub: string; // 身份证号码
  realName: string; // 真实姓名
  exp: number; // 过期时间
}

/**
 * 获取当前水印配置
 */
export const getWatermarkConfig = () => {
  const enabledStr = localStorage.getItem('wm_watermark_enabled');
  const enabled = enabledStr === null ? true : enabledStr === 'true';
  const opacityStr = localStorage.getItem('wm_watermark_opacity');
  const opacity = opacityStr ? parseFloat(opacityStr) : 0.16;
  const customText = localStorage.getItem('wm_watermark_custom_text') || '';
  return { enabled, opacity, customText };
};

/**
 * 核心水印绘制和挂载逻辑
 *
 * @param el 挂载水印的目标 DOM 元素
 * @param _binding Vue 指令绑定对象
 */
const addWatermark = (el: HTMLElement, _binding?: DirectiveBinding) => {
  const config = getWatermarkConfig();

  if (!config.enabled || config.opacity <= 0) {
    el.style.backgroundImage = 'none';
    el.style.display = 'none';
    return;
  }

  const token = localStorage.getItem('wmdb_token');
  let watermarkText = '稳骥 PacerSQL 安全审计';

  if (token) {
    try {
      const decoded = jwtDecode<DecodedToken>(token);
      const realName = decoded.realName || '未知用户';
      const idCard = decoded.sub || '000000000000000000';
      const lastFour = idCard.slice(-4);
      watermarkText = `${realName}_${lastFour}`;
    } catch (e) {
      console.error('Failed to decode JWT for watermark', e);
    }
  }

  if (config.customText && config.customText.trim()) {
    watermarkText = `${watermarkText} · ${config.customText.trim()}`;
  }

  // Create canvas to draw text
  const canvas = document.createElement('canvas');
  canvas.width = 280;
  canvas.height = 160;

  const ctx = canvas.getContext('2d');
  if (ctx) {
    ctx.rotate((-20 * Math.PI) / 180);
    ctx.font = '15px "PingFang SC", "Microsoft YaHei", sans-serif';
    ctx.fillStyle = `rgba(160, 160, 160, ${config.opacity})`;
    ctx.textAlign = 'left';
    ctx.textBaseline = 'middle';
    ctx.fillText(watermarkText, canvas.width / 8, canvas.height / 2);
  }

  // Set as background
  const base64Url = canvas.toDataURL('image/png');
  el.style.backgroundImage = `url('${base64Url}')`;
  el.style.backgroundRepeat = 'repeat';
  el.style.pointerEvents = 'none'; // Prevent watermark from blocking clicks
  el.style.position = 'absolute';
  el.style.top = '0';
  el.style.left = '0';
  el.style.width = '100%';
  el.style.height = '100%';
  el.style.zIndex = '9999';
  el.style.display = 'block';
  el.style.opacity = '1';

  // Anti-tamper using MutationObserver
  const observer = new MutationObserver((mutations) => {
    for (const mutation of mutations) {
      if (mutation.type === 'attributes' && mutation.attributeName === 'style') {
        const curConfig = getWatermarkConfig();
        if (curConfig.enabled && curConfig.opacity > 0) {
          el.style.backgroundImage = `url('${base64Url}')`;
          el.style.display = 'block';
          el.style.visibility = 'visible';
          el.style.opacity = '1';
          el.style.zIndex = '9999';
        }
      }
    }
  });

  observer.observe(el, { attributes: true, attributeFilter: ['style', 'class'] });
  (el as any).__wmdbObserver__ = observer;
  (el as any).__updateWatermark__ = () => addWatermark(el, _binding);
};

// 全局监听水印配置变更事件
if (typeof window !== 'undefined') {
  window.addEventListener('wm-watermark-update', () => {
    const list = document.querySelectorAll('.wmdb-watermark-layer');
    list.forEach((layer) => {
      const updateFn = (layer as any).__updateWatermark__;
      if (typeof updateFn === 'function') {
        updateFn();
      }
    });
  });
}

/**
 * 暴露给 Vue 的自定义指令对象
 */
export const watermark: Directive = {
  mounted(el, binding) {
    const container = el as HTMLElement;
    container.style.position = container.style.position || 'relative';

    const watermarkDiv = document.createElement('div');
    watermarkDiv.className = 'wmdb-watermark-layer';
    addWatermark(watermarkDiv, binding);
    container.appendChild(watermarkDiv);

    (container as any).__watermarkDiv__ = watermarkDiv;
  },
  unmounted(el) {
    const container = el as any;
    if (container.__watermarkDiv__) {
      const observer = container.__watermarkDiv__.__wmdbObserver__;
      if (observer) {
        observer.disconnect();
      }
      if (container.__watermarkDiv__.parentNode) {
        container.__watermarkDiv__.parentNode.removeChild(container.__watermarkDiv__);
      }
    }
  }
};
