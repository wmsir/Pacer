<template>
  <div class="portal-container">
    <!-- 背景流光与科技网格 -->
    <div class="ambient-sphere sphere-1"></div>
    <div class="ambient-sphere sphere-2"></div>
    <div class="ambient-sphere sphere-3"></div>
    <div class="portal-grid-bg"></div>

    <!-- Apple 风格悬浮磨砂顶栏 -->
    <header class="portal-nav" :class="{ 'scrolled': isScrolled }">
      <div class="nav-inner">
        <div class="nav-brand" @click="scrollTo('hero')">
          <div class="brand-badge-icon">
            <span class="horse-icon">🐎</span>
          </div>
          <div class="brand-text">
            <span class="brand-title">FWPT<span class="brand-dot">.</span>CN</span>
            <span class="brand-sub">开放基础平台</span>
          </div>
        </div>

        <nav class="nav-links">
          <a href="#hero" class="nav-link active" @click.prevent="scrollTo('hero')">首页</a>
          <a href="#pacersql" class="nav-link" @click.prevent="scrollTo('pacersql')">稳骥 PacerSQL</a>
          <a href="#features" class="nav-link" @click.prevent="scrollTo('features')">核心优势</a>
          <a href="#matrix" class="nav-link" @click.prevent="scrollTo('matrix')">平台矩阵</a>
          <a href="#nodes" class="nav-link" @click.prevent="scrollTo('nodes')">多机拓扑</a>
        </nav>

        <div class="nav-actions">
          <button class="btn-search-trigger" @click="focusSearch" title="快捷搜索 (按 /)">
            <el-icon><Search /></el-icon>
            <span class="search-tip">搜索平台...</span>
            <kbd class="kbd-shortcut">/</kbd>
          </button>
          <button class="btn-enter-pacer" @click="jumpToPacer">
            <span>进入 稳骥 PacerSQL</span>
            <el-icon><ArrowRight /></el-icon>
          </button>
        </div>
      </div>
    </header>

    <!-- 主视觉 Hero 区域 -->
    <main class="portal-content">
      <section id="hero" class="hero-section">
        <div class="hero-badge">
          <span class="badge-dot"></span>
          <span class="badge-text">FWPT 旗舰产品 · 稳骥 PacerSQL 全新发布</span>
        </div>

        <h1 class="hero-title">
          “骥”为千里良驹<br />
          <span class="gradient-text">“稳”压倒一切</span>
        </h1>

        <p class="hero-subtitle">
          直击生产高可用痛点。受专业平稳步态训练之赛马（Pacer），赋能企业级数据库变更自治与敏捷协同。<br />
          以 AI 深度预检、SpEL 智能路由与灰度分批流式引擎，护航百万级关键业务平稳运行。
        </p>

        <!-- 聚合智能搜索台 (Spotlight Omnibox) -->
        <div class="search-stage-wrap">
          <div class="search-box-container" :class="{ 'has-focus': isSearchFocused }">
            <div class="search-input-prefix">
              <el-icon class="search-main-icon"><Search /></el-icon>
            </div>
            <input
              ref="searchInputRef"
              v-model="searchQuery"
              type="text"
              class="search-omnibox-input"
              placeholder="搜索项目、工单审批、数据查询、多机节点或功能模块..."
              @focus="isSearchFocused = true"
              @blur="handleSearchBlur"
              @keydown.enter="handleSearchEnter"
              @keydown.down.prevent="navigateSearchResults(1)"
              @keydown.up.prevent="navigateSearchResults(-1)"
              @keydown.esc="searchQuery = ''; isSearchFocused = false"
            />
            <button v-if="searchQuery" class="clear-btn" @click="searchQuery = ''">
              <el-icon><Close /></el-icon>
            </button>
            <div class="search-action-btn" @click="handleSearchEnter">
              <span>跳转访问</span>
              <el-icon><Position /></el-icon>
            </div>

            <!-- 实时智能检索下拉菜单 -->
            <transition name="dropdown-fade">
              <div v-if="isSearchFocused && filteredItems.length > 0" class="search-dropdown-results">
                <div class="dropdown-header">
                  <span>匹配到 {{ filteredItems.length }} 个平台功能 / 模块</span>
                  <span class="dropdown-tip">按 Enter 直接跳转</span>
                </div>
                <div class="results-list">
                  <div
                    v-for="(item, idx) in filteredItems"
                    :key="item.id"
                    class="result-item"
                    :class="{ 'is-selected': selectedIndex === idx }"
                    @mousedown="selectItem(item)"
                    @mouseover="selectedIndex = idx"
                  >
                    <div class="item-icon-wrap" :style="{ background: item.colorBg }">
                      <span>{{ item.icon }}</span>
                    </div>
                    <div class="item-info">
                      <div class="item-title-row">
                        <span class="item-title">{{ item.name }}</span>
                        <span class="item-tag">{{ item.category }}</span>
                      </div>
                      <div class="item-desc">{{ item.desc }}</div>
                    </div>
                    <div class="item-action">
                      <span class="action-label">{{ item.targetLabel }}</span>
                      <el-icon class="action-arrow"><ArrowRight /></el-icon>
                    </div>
                  </div>
                </div>
              </div>
            </transition>
          </div>

          <!-- 热门快捷标签 -->
          <div class="hot-search-tags">
            <span class="hot-label">快捷直达：</span>
            <span
              v-for="tag in hotTags"
              :key="tag.text"
              class="tag-pill"
              @click="quickJumpTag(tag)"
            >
              {{ tag.text }}
            </span>
          </div>
        </div>

        <!-- 旗舰核心 CTA -->
        <div class="hero-cta-group">
          <button class="cta-primary-btn" @click="jumpToPacer">
            <span class="btn-shine"></span>
            <span class="horse-emoji">🐎</span>
            <span class="btn-text">立即进入 稳骥 PacerSQL</span>
            <span class="domain-badge">pacer.fwpt.cn</span>
          </button>
          <button class="cta-secondary-btn" @click="scrollTo('pacersql')">
            <span>探索技术架构</span>
            <el-icon><ArrowDown /></el-icon>
          </button>
        </div>
      </section>

      <!-- 稳骥 PacerSQL 旗舰产品展台 (Apple Keynote 级界面展示) -->
      <section id="pacersql" class="pacersql-showcase-section">
        <div class="section-badge">FLAGSHIP PLATFORM</div>
        <h2 class="section-title">稳骥 PacerSQL · 步态平稳，千里良驹</h2>
        <p class="section-desc">
          专为高可用研发运维而生。解决传统数据库变更中“长锁表阻断业务、从库严重延迟、审批脱节、权限失控”四大顽疾。
        </p>

        <!-- 科技感中控台拟态展示卡 -->
        <div class="mockup-glass-card">
          <div class="mockup-header-bar">
            <div class="window-controls">
              <span class="ctrl-dot red"></span>
              <span class="ctrl-dot yellow"></span>
              <span class="ctrl-dot green"></span>
            </div>
            <div class="mockup-url-pill">
              <span class="lock-icon">🔒</span>
              <span class="url-text">https://pacer.fwpt.cn/dashboard</span>
              <span class="status-live-tag">LIVE CLUSTER</span>
            </div>
            <div class="mockup-meta-info">
              <span>稳骥自治引擎 v2.8</span>
            </div>
          </div>

          <div class="mockup-body">
            <!-- 顶部集群与指标状态 -->
            <div class="dashboard-glance-grid">
              <div class="glance-card">
                <div class="glance-label">平台健康状态</div>
                <div class="glance-value text-emerald">100.0% 稳态</div>
                <div class="glance-sub">双机分布式守护在线</div>
              </div>
              <div class="glance-card">
                <div class="glance-label">Pacer 灰度流式步态</div>
                <div class="glance-value text-blue">0 锁表 / 0 抖动</div>
                <div class="glance-sub">微批渐进式流式提交</div>
              </div>
              <div class="glance-card">
                <div class="glance-label">AI 智能预检防线</div>
                <div class="glance-value text-purple">100% 覆盖</div>
                <div class="glance-sub">AST 语法与执行计划推演</div>
              </div>
              <div class="glance-card">
                <div class="glance-label">安全合规与审计</div>
                <div class="glance-value text-amber">国密 SM2/SM4</div>
                <div class="glance-sub">动态脱敏 & 全量操作回溯</div>
              </div>
            </div>

            <!-- 模拟工单流式执行控制台 -->
            <div class="stream-console-simulation">
              <div class="console-left">
                <div class="console-title-row">
                  <span class="pulse-emerald"></span>
                  <span class="console-title">Pacer 匀速分批流式执行引擎 (Real-time Stream)</span>
                  <span class="console-tag">工单 #1024 正在安全执行</span>
                </div>
                <div class="console-progress-track">
                  <div class="progress-fill" style="width: 78%;">
                    <div class="progress-glare"></div>
                  </div>
                </div>
                <div class="progress-stats">
                  <span>当前分批：第 78 / 100 批次 (每批 2,000 行)</span>
                  <span>锁等待：0.00ms · 从库延迟：&lt; 5ms</span>
                </div>
                <div class="console-log-lines">
                  <div class="log-line text-muted">[2026-09-06 00:30:01] 引擎启动：PacerSQL 动态计算最佳批次步态：batch_size=2000, sleep_ms=30</div>
                  <div class="log-line text-emerald">[2026-09-06 00:30:12] SpEL 智能路由：写入主库 (101.35.100.169)，读取校验路由至从库 (39.97.158.22)</div>
                  <div class="log-line text-cyan">[2026-09-06 00:30:25] AI 静态审计：语法检验 100 分，无全表更新高危风险，无阻塞索引</div>
                  <div class="log-line text-white">[2026-09-06 00:30:42] 灰度执行中：已安全写入 156,000 条记录，业务吞吐正常，无任何性能抖动</div>
                </div>
              </div>
              <div class="console-right">
                <div class="node-status-title">集群活跃拓扑</div>
                <div class="node-mini-card active">
                  <div class="node-mini-header">
                    <span class="node-tag-pri">MASTER 读写</span>
                    <span class="node-ip">101.35.100.169</span>
                  </div>
                  <div class="node-mini-info">北京主调度中心 · 延时 1ms</div>
                </div>
                <div class="node-mini-card edge">
                  <div class="node-mini-header">
                    <span class="node-tag-sec">EDGE 只读/加速</span>
                    <span class="node-ip">39.97.158.22</span>
                  </div>
                  <div class="node-mini-info">华北分布式节点 · 延时 2ms</div>
                </div>
                <button class="btn-console-enter" @click="jumpToPacer">
                  <span>进入实时管控台</span>
                  <el-icon><TopRight /></el-icon>
                </button>
              </div>
            </div>
          </div>
        </div>
      </section>

      <!-- 核心技术四大支柱 (Xiaomi / Apple Bento Grid 风格) -->
      <section id="features" class="bento-features-section">
        <div class="section-badge">CORE CAPABILITIES</div>
        <h2 class="section-title">极致稳健，直击高可用痛点</h2>
        <p class="section-desc">从语法预检、审批流转，到流式变更、安全审计，全流程环环相扣。</p>

        <div class="bento-grid">
          <!-- 卡片 1: Pacer 流式步态 -->
          <div class="bento-card bento-card-large card-horse">
            <div class="card-bg-glow"></div>
            <div class="card-icon-pill">
              <span>🐎</span>
              <span>PACER STREAM ENGINE</span>
            </div>
            <h3 class="card-title">Pacer 匀速流式步态引擎</h3>
            <p class="card-desc">
              如受训赛马般步伐平稳匀速。告别传统批量 DDL/DML 导致的长锁表与主从复制延迟。系统自动基于当前负载动态计算切片大小，配合微秒级休眠策略平滑入库，保障线上高频读写业务零感无抖动。
            </p>
            <div class="bento-metric-row">
              <div class="bento-metric">
                <span class="num">0<small>ms</small></span>
                <span class="lbl">业务锁表时间</span>
              </div>
              <div class="bento-metric">
                <span class="num">100<small>%</small></span>
                <span class="lbl">平滑下发保真</span>
              </div>
              <div class="bento-metric">
                <span class="num">100<small>万+</small></span>
                <span class="lbl">单表稳定吞吐</span>
              </div>
            </div>
          </div>

          <!-- 卡片 2: AI 预检 -->
          <div class="bento-card card-ai">
            <div class="card-icon-pill">
              <span>🧠</span>
              <span>AI DEEP REVIEW</span>
            </div>
            <h3 class="card-title">AI 智能预检与安全网关</h3>
            <p class="card-desc">
              集成大模型与规则引擎双重防线。精准识别无索引更新、全表扫描、隐式转换等隐患，秒级生成风险评估报告与优化建议。
            </p>
            <div class="card-pills-wrap">
              <span class="small-pill">语法 AST 树解析</span>
              <span class="small-pill">Dry-Run 真实影响推导</span>
              <span class="small-pill">高危 SQL 毫秒阻断</span>
            </div>
          </div>

          <!-- 卡片 3: SpEL 多机智能路由 -->
          <div class="bento-card card-routing">
            <div class="card-icon-pill">
              <span>🌐</span>
              <span>SpEL DYNAMIC ROUTING</span>
            </div>
            <h3 class="card-title">多节点协同与主从读写分离</h3>
            <p class="card-desc">
              支持多台异地物理机与云主机无缝入网。根据 SQL 操作类型智能路由（查询走从节点、发布走主节点），故障自动熔断降级。
            </p>
            <div class="card-pills-wrap">
              <span class="small-pill">主从读写自动分流</span>
              <span class="small-pill">SpEL 表达式规则</span>
              <span class="small-pill">节点健康实时心跳</span>
            </div>
          </div>

          <!-- 卡片 4: 安全脱敏与全渠道告警 -->
          <div class="bento-card bento-card-large card-security">
            <div class="card-icon-pill">
              <span>🛡️</span>
              <span>SECURITY & NOTIFICATIONS</span>
            </div>
            <h3 class="card-title">全量合规审计、动态脱敏与多厂商告警</h3>
            <p class="card-desc">
              敏感字段动态掩码（身份证、手机号等），页面防截屏数字盲水印；支持短信（阿里云/腾讯云）、企业微信、钉钉、飞书与电话多维告警矩阵，审批工单实时催办。
            </p>
            <div class="bento-metric-row">
              <div class="bento-metric">
                <span class="num">&lt; 3<small>s</small></span>
                <span class="lbl">消息全渠道触达</span>
              </div>
              <div class="bento-metric">
                <span class="num">100<small>%</small></span>
                <span class="lbl">全量 SQL 审计追踪</span>
              </div>
              <div class="bento-metric">
                <span class="num">国密<small>合规</small></span>
                <span class="lbl">SM2/SM4 传输加密</span>
              </div>
            </div>
          </div>
        </div>
      </section>

      <!-- 多机拓扑总览 (Nodes Section) -->
      <section id="nodes" class="nodes-topology-section">
        <div class="section-badge">DISTRIBUTED NODES</div>
        <h2 class="section-title">高可用多机协同拓扑</h2>
        <p class="section-desc">已实现多机器多实例统一调度监控，支持主节点写、从节点读的生产高可用架构。</p>

        <div class="nodes-display-grid">
          <div class="node-card master-node">
            <div class="node-card-badge">PRIMARY 写入中心</div>
            <div class="node-card-header">
              <div class="node-icon">🖥️</div>
              <div>
                <div class="node-name">北京主控中心集群</div>
                <div class="node-ip-text">101.35.100.169:80</div>
              </div>
            </div>
            <div class="node-desc">
              承载核心工单审批、数据库元数据编排、写入及变更下发，内置 Nginx 高性能反向代理与 MySQL 8.0 存储底座。
            </div>
            <div class="node-tags">
              <span class="ntag">读写分离: WRITE</span>
              <span class="ntag">Nginx 泛域名绑定</span>
              <span class="ntag">已挂载</span>
            </div>
          </div>

          <div class="node-card worker-node">
            <div class="node-card-badge">SLAVE 只读与计算加速</div>
            <div class="node-card-header">
              <div class="node-icon">⚡</div>
              <div>
                <div class="node-name">华北分布式计算节点</div>
                <div class="node-ip-text">39.97.158.22:80</div>
              </div>
            </div>
            <div class="node-desc">
              承载只读数据查询、数据脱敏计算、AI 语法解析加速。支持与主控中心实时双向探活与负载分流。
            </div>
            <div class="node-tags">
              <span class="ntag">读写分离: READ_ONLY</span>
              <span class="ntag">2GB 弹性虚拟交换</span>
              <span class="ntag">已挂载</span>
            </div>
          </div>
        </div>
      </section>

      <!-- FWPT 平台生态矩阵 (Platform Matrix) -->
      <section id="matrix" class="matrix-section">
        <div class="section-badge">FWPT ECOSYSTEM</div>
        <h2 class="section-title">FWPT 开放基础平台矩阵</h2>
        <p class="section-desc">构建面向未来的敏捷研发、智能数据与微服务治理统一数字底座。</p>

        <div class="matrix-grid">
          <div class="matrix-card active-card" @click="jumpToPacer">
            <div class="matrix-status-tag active">核心运营中</div>
            <div class="matrix-icon">🐎</div>
            <h3 class="matrix-title">稳骥 PacerSQL</h3>
            <div class="matrix-domain">pacer.fwpt.cn</div>
            <p class="matrix-desc">企业级数据库协同治理与高可用变更自治平台。千里良驹，稳压倒一切。</p>
            <div class="matrix-cta">
              <span>立即进入系统</span>
              <el-icon><ArrowRight /></el-icon>
            </div>
          </div>

          <div class="matrix-card coming-card">
            <div class="matrix-status-tag plan">规划中</div>
            <div class="matrix-icon">📡</div>
            <h3 class="matrix-title">灵犀 Monitor</h3>
            <div class="matrix-domain">monitor.fwpt.cn</div>
            <p class="matrix-desc">全栈多维可观测与智能 APM 监控分析中枢，秒级捕获慢调用与异常瓶颈。</p>
            <div class="matrix-cta disabled">
              <span>敬请期待</span>
            </div>
          </div>

          <div class="matrix-card coming-card">
            <div class="matrix-status-tag plan">规划中</div>
            <div class="matrix-icon">🛡️</div>
            <h3 class="matrix-title">伏羲 Gateway</h3>
            <div class="matrix-domain">gateway.fwpt.cn</div>
            <p class="matrix-desc">零信任安全服务网关，提供动态限流、国密安全传输与细粒度访问控制。</p>
            <div class="matrix-cta disabled">
              <span>敬请期待</span>
            </div>
          </div>

          <div class="matrix-card coming-card">
            <div class="matrix-status-tag plan">规划中</div>
            <div class="matrix-icon">🌊</div>
            <h3 class="matrix-title">河图 DataOps</h3>
            <div class="matrix-domain">data.fwpt.cn</div>
            <p class="matrix-desc">现代化流批一体数据管道与指标治理引擎，打通跨源数据协同闭环。</p>
            <div class="matrix-cta disabled">
              <span>敬请期待</span>
            </div>
          </div>
        </div>
      </section>

      <!-- 底部大横幅 CTA -->
      <section class="bottom-banner-section">
        <div class="banner-inner">
          <div class="banner-badge">立即开启高可用变更之旅</div>
          <h2 class="banner-title">准备好让生产数据库稳如泰山了吗？</h2>
          <p class="banner-subtitle">
            无论您是 DBA、研发团队还是架构师，稳骥 PacerSQL 都能为您带来前所未有的安全感与敏捷体验。
          </p>
          <div class="banner-buttons">
            <button class="banner-primary-btn" @click="jumpToPacer">
              <span>进入 稳骥 PacerSQL (pacer.fwpt.cn)</span>
              <el-icon><ArrowRight /></el-icon>
            </button>
            <button class="banner-ghost-btn" @click="scrollTo('hero')">
              <span>返回顶部</span>
              <el-icon><Top /></el-icon>
            </button>
          </div>
        </div>
      </section>
    </main>

    <!-- Apple 风格精简页脚 -->
    <footer class="portal-footer">
      <div class="footer-inner">
        <div class="footer-top-row">
          <div class="footer-brand-col">
            <div class="f-brand">FWPT<span class="f-dot">.</span>CN</div>
            <p class="f-desc">“骥”为千里良驹，“稳”压倒一切。为企业级数字化基础设施提供极致稳健的高可用治理能力。</p>
          </div>
          <div class="footer-links-col">
            <div class="f-col-title">平台生态</div>
            <a href="javascript:;" @click="jumpToPacer">稳骥 PacerSQL</a>
            <a href="javascript:;" @click="jumpToModule('/ticket-list')">工单审批流</a>
            <a href="javascript:;" @click="jumpToModule('/data-query')">数据查询与脱敏</a>
            <a href="javascript:;" @click="jumpToModule('/audit-dashboard')">全量审计大盘</a>
          </div>
          <div class="footer-links-col">
            <div class="f-col-title">技术特性</div>
            <a href="#features" @click.prevent="scrollTo('features')">Pacer 流式引擎</a>
            <a href="#features" @click.prevent="scrollTo('features')">AI 深度预检</a>
            <a href="#nodes" @click.prevent="scrollTo('nodes')">SpEL 读写分离路由</a>
            <a href="#features" @click.prevent="scrollTo('features')">国密合规与数字水印</a>
          </div>
          <div class="footer-links-col">
            <div class="f-col-title">节点集群</div>
            <a href="javascript:;" @click="jumpToNode('101.35.100.169')">北京主控节点 (101.35.100.169)</a>
            <a href="javascript:;" @click="jumpToNode('39.97.158.22')">华北计算节点 (39.97.158.22)</a>
            <a href="javascript:;" @click="jumpToModule('/dashboard')">集群全局拓扑</a>
          </div>
        </div>

        <div class="footer-bottom-bar">
          <div class="copyright">
            © 2026 fwpt.cn · 稳骥 PacerSQL 版权所有 · 企业级高可用数据库变更自治云平台
          </div>
          <div class="footer-badges">
            <span>粤ICP备2025000000号-1</span>
            <span>·</span>
            <span>TLS 1.3 传输加密</span>
            <span>·</span>
            <span>国密 SM2/SM4 认证</span>
          </div>
        </div>
      </div>
    </footer>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import {
  Search, ArrowRight, ArrowDown, Close, Position, TopRight, Top
} from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'

const router = useRouter()

const isScrolled = ref(false)
const searchQuery = ref('')
const isSearchFocused = ref(false)
const searchInputRef = ref<HTMLInputElement | null>(null)
const selectedIndex = ref(0)

// 监听滚动以改变导航栏透明度
const handleScroll = () => {
  isScrolled.value = window.scrollY > 20
}

onMounted(() => {
  window.addEventListener('scroll', handleScroll)
  window.addEventListener('keydown', handleGlobalKeydown)
})

onUnmounted(() => {
  window.removeEventListener('scroll', handleScroll)
  window.removeEventListener('keydown', handleGlobalKeydown)
})

// 快捷键聚焦搜索框
const handleGlobalKeydown = (e: KeyboardEvent) => {
  if (e.key === '/' && document.activeElement !== searchInputRef.value) {
    e.preventDefault()
    focusSearch()
  }
}

const focusSearch = () => {
  isSearchFocused.value = true
  searchInputRef.value?.focus()
}

const handleSearchBlur = () => {
  setTimeout(() => {
    isSearchFocused.value = false
  }, 200)
}

// 锚点平滑滚动
const scrollTo = (id: string) => {
  const el = document.getElementById(id)
  if (el) {
    el.scrollIntoView({ behavior: 'smooth' })
  }
}

// 快捷直达热门标签
const hotTags = [
  { text: '稳骥 PacerSQL', target: 'pacersql' },
  { text: 'SQL上线工单', target: '/ticket-list' },
  { text: '数据查询与脱敏', target: '/data-query' },
  { text: '多机主从读写分离', target: '/dashboard' },
  { text: '短信/企微告警', target: '/notification-config' },
  { text: '全量审计追踪', target: '/audit-dashboard' }
]

// 搜索索引库
interface SearchItem {
  id: string
  name: string
  category: string
  desc: string
  icon: string
  colorBg: string
  routePath?: string
  externalDomain?: string
  targetLabel: string
}

const searchableCatalog: SearchItem[] = [
  {
    id: 'pacer-main',
    name: '稳骥 PacerSQL 核心平台',
    category: '旗舰产品',
    desc: '千里良驹，稳压倒一切。高可用数据库协同治理与变更自治平台',
    icon: '🐎',
    colorBg: 'linear-gradient(135deg, #2563eb, #3b82f6)',
    externalDomain: 'pacer.fwpt.cn',
    routePath: '/login',
    targetLabel: '直达平台'
  },
  {
    id: 'ticket-center',
    name: 'SQL 变更工单中心',
    category: '变更管控',
    desc: 'SQL 提交、AI 语法预检、Dry-Run 影响推导与多级审批流转',
    icon: '📝',
    colorBg: 'linear-gradient(135deg, #059669, #10b981)',
    routePath: '/ticket-list',
    targetLabel: '进入工单'
  },
  {
    id: 'data-query',
    name: '在线数据查询与动态脱敏',
    category: '数据资产',
    desc: 'Web SQL 查询执行器、敏感字段自动掩码与数字水印防护',
    icon: '🔍',
    colorBg: 'linear-gradient(135deg, #7c3aed, #8b5cf6)',
    routePath: '/data-query',
    targetLabel: '安全查询'
  },
  {
    id: 'nodes-cluster',
    name: '多机分布式集群总览',
    category: '高可用拓扑',
    desc: '101.35.100.169 (主) 与 39.97.158.22 (从) 多节点状态与读写分离',
    icon: '🖥️',
    colorBg: 'linear-gradient(135deg, #d97706, #f59e0b)',
    routePath: '/dashboard',
    targetLabel: '查看集群'
  },
  {
    id: 'audit-board',
    name: '安全合规与操作审计大盘',
    category: '安全审计',
    desc: '全量 SQL 拦截日志、IP 白名单、高危指令拦截报表导出',
    icon: '🛡️',
    colorBg: 'linear-gradient(135deg, #dc2626, #ef4444)',
    routePath: '/audit-dashboard',
    targetLabel: '合规审计'
  },
  {
    id: 'notice-center',
    name: '消息通知与告警通道中心',
    category: '消息总线',
    desc: '短信 (阿里/腾讯)、企业微信、钉钉、飞书与电话告警配置',
    icon: '🔔',
    colorBg: 'linear-gradient(135deg, #0284c7, #0ea5e9)',
    routePath: '/notification-config',
    targetLabel: '告警配置'
  },
  {
    id: 'ai-review',
    name: 'AI SQL 智能分析与优化',
    category: '智能预检',
    desc: '基于大模型的 SQL 索引推荐、执行计划慢查诊断与重构建议',
    icon: '🧠',
    colorBg: 'linear-gradient(135deg, #9333ea, #a855f7)',
    routePath: '/ai-review',
    targetLabel: 'AI 预检'
  }
]

// 搜索过滤
const filteredItems = computed(() => {
  if (!searchQuery.value.trim()) {
    return searchableCatalog.slice(0, 5)
  }
  const q = searchQuery.value.toLowerCase().trim()
  return searchableCatalog.filter(
    item =>
      item.name.toLowerCase().includes(q) ||
      item.desc.toLowerCase().includes(q) ||
      item.category.toLowerCase().includes(q)
  )
})

const navigateSearchResults = (delta: number) => {
  const max = filteredItems.value.length
  if (max === 0) return
  selectedIndex.value = (selectedIndex.value + delta + max) % max
}

// 核心跳转逻辑（自适应当前访问域名）
const jumpToPacer = () => {
  const host = window.location.hostname
  const port = window.location.port ? `:${window.location.port}` : ''
  // 若用户通过 fwpt.cn 访问，优先跳转到二级域名 pacer.fwpt.cn
  if (host === 'fwpt.cn' || host === 'www.fwpt.cn') {
    window.location.href = `${window.location.protocol}//pacer.fwpt.cn${port}/login`
    return
  }
  // 否则在当前环境（如 IP 或 localhost）直接进入登录或控制台
  router.push('/login')
}

const selectItem = (item: SearchItem) => {
  if (item.externalDomain && (window.location.hostname === 'fwpt.cn' || window.location.hostname === 'www.fwpt.cn')) {
    const port = window.location.port ? `:${window.location.port}` : ''
    window.location.href = `${window.location.protocol}//${item.externalDomain}${port}${item.routePath || '/login'}`
    return
  }
  if (item.routePath) {
    router.push(item.routePath)
  } else {
    jumpToPacer()
  }
}

const handleSearchEnter = () => {
  if (filteredItems.value.length > 0) {
    selectItem(filteredItems.value[selectedIndex.value] || filteredItems.value[0])
  } else {
    jumpToPacer()
  }
}

const quickJumpTag = (tag: { text: string; target: string }) => {
  if (tag.target === 'pacersql') {
    jumpToPacer()
  } else if (tag.target.startsWith('/')) {
    router.push(tag.target)
  }
}

const jumpToModule = (path: string) => {
  router.push(path)
}

const jumpToNode = (ip: string) => {
  ElMessage.info(`正在连接节点 ${ip} ...`)
  router.push('/dashboard')
}
</script>

<style scoped>
/* 全局暗色奢华基底 */
.portal-container {
  min-height: 100vh;
  background-color: #05070f;
  color: #f1f5f9;
  font-family: -apple-system, BlinkMacSystemFont, "SF Pro Display", "SF Pro Text", "Segoe UI", Roboto, "Helvetica Neue", sans-serif;
  overflow-x: hidden;
  position: relative;
  selection-background-color: #3b82f6;
}

/* 环境背景流光球 */
.ambient-sphere {
  position: absolute;
  border-radius: 50%;
  filter: blur(140px);
  pointer-events: none;
  z-index: 0;
  opacity: 0.35;
}
.sphere-1 {
  top: -100px;
  left: 20%;
  width: 600px;
  height: 600px;
  background: radial-gradient(circle, #1e40af 0%, rgba(30, 64, 175, 0) 70%);
}
.sphere-2 {
  top: 300px;
  right: 5%;
  width: 500px;
  height: 500px;
  background: radial-gradient(circle, #0284c7 0%, rgba(2, 132, 199, 0) 70%);
}
.sphere-3 {
  top: 1200px;
  left: 10%;
  width: 700px;
  height: 700px;
  background: radial-gradient(circle, #4338ca 0%, rgba(67, 56, 202, 0) 70%);
}

.portal-grid-bg {
  position: absolute;
  inset: 0;
  background-image: linear-gradient(to right, rgba(255, 255, 255, 0.02) 1px, transparent 1px),
                    linear-gradient(to bottom, rgba(255, 255, 255, 0.02) 1px, transparent 1px);
  background-size: 60px 60px;
  pointer-events: none;
  z-index: 0;
}

/* Apple 风格磨砂悬浮顶栏 */
.portal-nav {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  height: 68px;
  z-index: 100;
  transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  background: rgba(5, 7, 15, 0.6);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}
.portal-nav.scrolled {
  background: rgba(5, 7, 15, 0.85);
  border-bottom: 1px solid rgba(255, 255, 255, 0.12);
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
}
.nav-inner {
  max-width: 1280px;
  margin: 0 auto;
  height: 100%;
  padding: 0 24px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.nav-brand {
  display: flex;
  align-items: center;
  gap: 12px;
  cursor: pointer;
}
.brand-badge-icon {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  background: linear-gradient(135deg, #1e3a8a, #2563eb);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  box-shadow: 0 4px 12px rgba(37, 99, 235, 0.4);
}
.brand-text {
  display: flex;
  flex-direction: column;
}
.brand-title {
  font-size: 18px;
  font-weight: 800;
  letter-spacing: -0.5px;
  color: #ffffff;
}
.brand-dot {
  color: #38bdf8;
}
.brand-sub {
  font-size: 11px;
  color: #94a3b8;
  letter-spacing: 0.5px;
}

.nav-links {
  display: flex;
  align-items: center;
  gap: 28px;
}
.nav-link {
  color: #94a3b8;
  font-size: 14px;
  font-weight: 500;
  text-decoration: none;
  transition: color 0.2s;
  padding: 6px 0;
}
.nav-link:hover, .nav-link.active {
  color: #ffffff;
}

.nav-actions {
  display: flex;
  align-items: center;
  gap: 16px;
}
.btn-search-trigger {
  display: flex;
  align-items: center;
  gap: 8px;
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.1);
  padding: 6px 14px;
  border-radius: 20px;
  color: #94a3b8;
  font-size: 13px;
  cursor: pointer;
  transition: all 0.2s;
}
.btn-search-trigger:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #ffffff;
  border-color: rgba(255, 255, 255, 0.2);
}
.kbd-shortcut {
  background: rgba(255, 255, 255, 0.12);
  color: #cbd5e1;
  padding: 1px 6px;
  border-radius: 6px;
  font-size: 11px;
  font-family: monospace;
}
.btn-enter-pacer {
  display: flex;
  align-items: center;
  gap: 8px;
  background: linear-gradient(135deg, #2563eb, #1d4ed8);
  color: #ffffff;
  border: none;
  padding: 8px 18px;
  border-radius: 20px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  box-shadow: 0 4px 14px rgba(37, 99, 235, 0.4);
  transition: all 0.2s;
}
.btn-enter-pacer:hover {
  transform: translateY(-1px);
  box-shadow: 0 6px 20px rgba(37, 99, 235, 0.6);
}

/* 主内容容器 */
.portal-content {
  position: relative;
  z-index: 1;
  max-width: 1280px;
  margin: 0 auto;
  padding: 0 24px;
}

/* Hero Section */
.hero-section {
  padding-top: 140px;
  padding-bottom: 80px;
  text-align: center;
  display: flex;
  flex-direction: column;
  align-items: center;
}
.hero-badge {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  background: rgba(37, 99, 235, 0.15);
  border: 1px solid rgba(59, 130, 246, 0.3);
  padding: 6px 16px;
  border-radius: 30px;
  font-size: 13px;
  color: #60a5fa;
  margin-bottom: 24px;
}
.badge-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #38bdf8;
  box-shadow: 0 0 10px #38bdf8;
  animation: pulse 2s infinite;
}
.hero-title {
  font-size: 64px;
  line-height: 1.15;
  font-weight: 900;
  letter-spacing: -1.5px;
  margin: 0 0 24px 0;
  color: #ffffff;
}
.gradient-text {
  background: linear-gradient(135deg, #60a5fa 10%, #38bdf8 50%, #a78bfa 90%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}
.hero-subtitle {
  max-width: 820px;
  font-size: 18px;
  line-height: 1.6;
  color: #94a3b8;
  margin: 0 0 44px 0;
}

/* 聚合智能搜索台 (Spotlight Omnibox) */
.search-stage-wrap {
  width: 100%;
  max-width: 760px;
  margin: 0 auto 36px auto;
  position: relative;
}
.search-box-container {
  display: flex;
  align-items: center;
  background: rgba(15, 23, 42, 0.75);
  border: 1px solid rgba(255, 255, 255, 0.14);
  border-radius: 18px;
  padding: 8px 10px 8px 20px;
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  box-shadow: 0 20px 50px rgba(0, 0, 0, 0.6);
  transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  position: relative;
}
.search-box-container.has-focus {
  border-color: #3b82f6;
  box-shadow: 0 0 0 4px rgba(59, 130, 246, 0.25), 0 20px 50px rgba(0, 0, 0, 0.7);
}
.search-input-prefix {
  display: flex;
  align-items: center;
  color: #94a3b8;
  font-size: 20px;
  margin-right: 14px;
}
.search-omnibox-input {
  flex: 1;
  background: transparent;
  border: none;
  outline: none;
  font-size: 16px;
  color: #ffffff;
  font-family: inherit;
}
.search-omnibox-input::placeholder {
  color: #64748b;
}
.clear-btn {
  background: transparent;
  border: none;
  color: #64748b;
  cursor: pointer;
  padding: 6px;
  display: flex;
  align-items: center;
}
.clear-btn:hover {
  color: #ffffff;
}
.search-action-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  background: linear-gradient(135deg, #2563eb, #1d4ed8);
  color: #ffffff;
  padding: 10px 20px;
  border-radius: 12px;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
}
.search-action-btn:hover {
  background: linear-gradient(135deg, #3b82f6, #2563eb);
}

/* 实时检索下拉项 */
.search-dropdown-results {
  position: absolute;
  top: calc(100% + 10px);
  left: 0;
  right: 0;
  background: #0f172a;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 16px;
  box-shadow: 0 24px 60px rgba(0, 0, 0, 0.8);
  overflow: hidden;
  z-index: 50;
  text-align: left;
}
.dropdown-header {
  padding: 12px 18px;
  background: rgba(255, 255, 255, 0.03);
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  font-size: 12px;
  color: #94a3b8;
  display: flex;
  justify-content: space-between;
}
.results-list {
  max-height: 380px;
  overflow-y: auto;
  padding: 8px;
}
.result-item {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 12px 14px;
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.15s;
}
.result-item.is-selected, .result-item:hover {
  background: rgba(255, 255, 255, 0.08);
}
.item-icon-wrap {
  width: 38px;
  height: 38px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  flex-shrink: 0;
}
.item-info {
  flex: 1;
  min-width: 0;
}
.item-title-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}
.item-title {
  font-size: 14px;
  font-weight: 600;
  color: #ffffff;
}
.item-tag {
  font-size: 11px;
  background: rgba(255, 255, 255, 0.1);
  padding: 1px 6px;
  border-radius: 4px;
  color: #94a3b8;
}
.item-desc {
  font-size: 12px;
  color: #94a3b8;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.item-action {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  color: #38bdf8;
  flex-shrink: 0;
}

/* 热门快捷直达标签 */
.hot-search-tags {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  flex-wrap: wrap;
  margin-top: 14px;
}
.hot-label {
  font-size: 13px;
  color: #64748b;
}
.tag-pill {
  font-size: 12px;
  color: #94a3b8;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.08);
  padding: 4px 12px;
  border-radius: 14px;
  cursor: pointer;
  transition: all 0.2s;
}
.tag-pill:hover {
  background: rgba(59, 130, 246, 0.15);
  border-color: rgba(59, 130, 246, 0.4);
  color: #38bdf8;
}

/* CTA 按钮组 */
.hero-cta-group {
  display: flex;
  align-items: center;
  gap: 20px;
}
.cta-primary-btn {
  position: relative;
  overflow: hidden;
  display: flex;
  align-items: center;
  gap: 12px;
  background: linear-gradient(135deg, #1d4ed8 0%, #2563eb 50%, #3b82f6 100%);
  color: #ffffff;
  border: none;
  padding: 14px 32px;
  border-radius: 30px;
  font-size: 16px;
  font-weight: 700;
  cursor: pointer;
  box-shadow: 0 10px 30px rgba(37, 99, 235, 0.5);
  transition: all 0.3s;
}
.cta-primary-btn:hover {
  transform: translateY(-2px);
  box-shadow: 0 14px 40px rgba(37, 99, 235, 0.7);
}
.horse-emoji {
  font-size: 20px;
}
.domain-badge {
  background: rgba(0, 0, 0, 0.25);
  border: 1px solid rgba(255, 255, 255, 0.2);
  padding: 2px 10px;
  border-radius: 12px;
  font-size: 12px;
  font-weight: 500;
  color: #93c5fd;
}
.cta-secondary-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.14);
  color: #ffffff;
  padding: 14px 26px;
  border-radius: 30px;
  font-size: 15px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
}
.cta-secondary-btn:hover {
  background: rgba(255, 255, 255, 0.12);
}

/* 通用 Section 头部 */
.section-badge {
  display: inline-block;
  font-size: 12px;
  letter-spacing: 2px;
  font-weight: 700;
  color: #38bdf8;
  margin-bottom: 12px;
  text-align: center;
}
.section-title {
  font-size: 40px;
  font-weight: 800;
  letter-spacing: -1px;
  margin: 0 0 16px 0;
  color: #ffffff;
  text-align: center;
}
.section-desc {
  max-width: 680px;
  margin: 0 auto 50px auto;
  font-size: 16px;
  line-height: 1.6;
  color: #94a3b8;
  text-align: center;
}

/* 拟态 Showcase 卡片 (Apple 级视窗) */
.pacersql-showcase-section {
  padding: 80px 0;
  text-align: center;
}
.mockup-glass-card {
  background: rgba(15, 23, 42, 0.7);
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 24px;
  overflow: hidden;
  box-shadow: 0 30px 80px rgba(0, 0, 0, 0.8), 0 0 80px rgba(37, 99, 235, 0.15);
  backdrop-filter: blur(24px);
  -webkit-backdrop-filter: blur(24px);
  text-align: left;
}
.mockup-header-bar {
  background: rgba(255, 255, 255, 0.04);
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  padding: 12px 20px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.window-controls {
  display: flex;
  gap: 8px;
}
.ctrl-dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
}
.ctrl-dot.red { background: #ef4444; }
.ctrl-dot.yellow { background: #f59e0b; }
.ctrl-dot.green { background: #10b981; }

.mockup-url-pill {
  display: flex;
  align-items: center;
  gap: 8px;
  background: rgba(0, 0, 0, 0.4);
  border: 1px solid rgba(255, 255, 255, 0.08);
  padding: 6px 18px;
  border-radius: 20px;
  font-size: 13px;
  color: #94a3b8;
}
.status-live-tag {
  background: rgba(16, 185, 129, 0.2);
  border: 1px solid rgba(16, 185, 129, 0.4);
  color: #34d399;
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 10px;
}
.mockup-meta-info {
  font-size: 12px;
  color: #64748b;
}

.mockup-body {
  padding: 28px;
}
.dashboard-glance-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 16px;
  margin-bottom: 24px;
}
.glance-card {
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 16px;
  padding: 18px 20px;
}
.glance-label {
  font-size: 12px;
  color: #94a3b8;
  margin-bottom: 8px;
}
.glance-value {
  font-size: 24px;
  font-weight: 800;
  margin-bottom: 4px;
}
.text-emerald { color: #10b981; }
.text-blue { color: #3b82f6; }
.text-purple { color: #a855f7; }
.text-amber { color: #f59e0b; }
.text-cyan { color: #06b6d4; }
.glance-sub {
  font-size: 12px;
  color: #64748b;
}

/* 流式模拟器 */
.stream-console-simulation {
  display: grid;
  grid-template-columns: 2fr 1fr;
  gap: 20px;
  background: rgba(0, 0, 0, 0.4);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 18px;
  padding: 24px;
}
.console-title-row {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 16px;
}
.pulse-emerald {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: #10b981;
  box-shadow: 0 0 12px #10b981;
  animation: pulse 1.5s infinite;
}
.console-title {
  font-size: 15px;
  font-weight: 700;
  color: #ffffff;
}
.console-tag {
  background: rgba(59, 130, 246, 0.2);
  color: #60a5fa;
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 6px;
}
.console-progress-track {
  width: 100%;
  height: 8px;
  background: rgba(255, 255, 255, 0.08);
  border-radius: 4px;
  overflow: hidden;
  margin-bottom: 10px;
}
.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #2563eb, #38bdf8);
  border-radius: 4px;
  position: relative;
}
.progress-glare {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.4), transparent);
  animation: glare 2s infinite;
}
.progress-stats {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
  color: #94a3b8;
  margin-bottom: 16px;
}
.console-log-lines {
  background: rgba(5, 7, 15, 0.8);
  border-radius: 10px;
  padding: 14px;
  font-family: "JetBrains Mono", Consolas, monospace;
  font-size: 12px;
  line-height: 1.7;
}
.log-line {
  word-break: break-all;
}
.text-muted { color: #64748b; }
.text-white { color: #f8fafc; }

.console-right {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.node-status-title {
  font-size: 13px;
  font-weight: 600;
  color: #cbd5e1;
  margin-bottom: 4px;
}
.node-mini-card {
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 12px;
  padding: 12px;
}
.node-mini-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 6px;
}
.node-tag-pri {
  font-size: 10px;
  background: rgba(16, 185, 129, 0.2);
  color: #34d399;
  padding: 2px 6px;
  border-radius: 4px;
  font-weight: 700;
}
.node-tag-sec {
  font-size: 10px;
  background: rgba(59, 130, 246, 0.2);
  color: #60a5fa;
  padding: 2px 6px;
  border-radius: 4px;
  font-weight: 700;
}
.node-ip {
  font-size: 12px;
  font-family: monospace;
  color: #ffffff;
}
.node-mini-info {
  font-size: 11px;
  color: #94a3b8;
}
.btn-console-enter {
  margin-top: auto;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.15);
  color: #ffffff;
  padding: 10px;
  border-radius: 10px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
}
.btn-console-enter:hover {
  background: rgba(59, 130, 246, 0.3);
  border-color: #3b82f6;
}

/* Bento Grid 区域 */
.bento-features-section {
  padding: 80px 0;
}
.bento-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 24px;
}
.bento-card {
  position: relative;
  background: rgba(15, 23, 42, 0.6);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 24px;
  padding: 36px;
  overflow: hidden;
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  transition: all 0.3s;
}
.bento-card:hover {
  border-color: rgba(59, 130, 246, 0.35);
  transform: translateY(-4px);
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.5);
}
.bento-card-large {
  grid-column: span 2;
}
.card-bg-glow {
  position: absolute;
  top: 0;
  right: 0;
  width: 300px;
  height: 300px;
  background: radial-gradient(circle, rgba(59, 130, 246, 0.15) 0%, transparent 70%);
  pointer-events: none;
}
.card-icon-pill {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  font-weight: 700;
  color: #38bdf8;
  background: rgba(56, 189, 248, 0.1);
  border: 1px solid rgba(56, 189, 248, 0.2);
  padding: 4px 12px;
  border-radius: 20px;
  margin-bottom: 20px;
}
.card-title {
  font-size: 24px;
  font-weight: 800;
  color: #ffffff;
  margin: 0 0 14px 0;
}
.card-desc {
  font-size: 15px;
  line-height: 1.6;
  color: #94a3b8;
  margin: 0 0 28px 0;
}
.bento-metric-row {
  display: flex;
  gap: 40px;
}
.bento-metric .num {
  display: block;
  font-size: 32px;
  font-weight: 900;
  color: #ffffff;
}
.bento-metric .num small {
  font-size: 16px;
  color: #38bdf8;
  margin-left: 2px;
}
.bento-metric .lbl {
  font-size: 12px;
  color: #64748b;
}
.card-pills-wrap {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.small-pill {
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
  padding: 4px 12px;
  border-radius: 8px;
  font-size: 12px;
  color: #cbd5e1;
}

/* 多机拓扑 */
.nodes-topology-section {
  padding: 80px 0;
  text-align: center;
}
.nodes-display-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 24px;
  max-width: 980px;
  margin: 0 auto;
  text-align: left;
}
.node-card {
  background: rgba(15, 23, 42, 0.7);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 20px;
  padding: 28px;
  position: relative;
  transition: all 0.3s;
}
.node-card:hover {
  border-color: #3b82f6;
  transform: translateY(-3px);
}
.node-card-badge {
  position: absolute;
  top: 20px;
  right: 20px;
  font-size: 11px;
  font-weight: 700;
  padding: 3px 10px;
  border-radius: 12px;
}
.master-node .node-card-badge {
  background: rgba(16, 185, 129, 0.2);
  color: #34d399;
}
.worker-node .node-card-badge {
  background: rgba(59, 130, 246, 0.2);
  color: #60a5fa;
}
.node-card-header {
  display: flex;
  align-items: center;
  gap: 16px;
  margin-bottom: 16px;
}
.node-icon {
  font-size: 32px;
}
.node-name {
  font-size: 18px;
  font-weight: 700;
  color: #ffffff;
}
.node-ip-text {
  font-size: 14px;
  font-family: monospace;
  color: #94a3b8;
}
.node-desc {
  font-size: 14px;
  color: #94a3b8;
  line-height: 1.6;
  margin-bottom: 20px;
}
.node-tags {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}
.ntag {
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.08);
  font-size: 11px;
  color: #cbd5e1;
  padding: 3px 8px;
  border-radius: 6px;
}

/* 平台矩阵 */
.matrix-section {
  padding: 80px 0;
  text-align: center;
}
.matrix-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 20px;
  text-align: left;
}
.matrix-card {
  background: rgba(15, 23, 42, 0.6);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 20px;
  padding: 28px 24px;
  position: relative;
  display: flex;
  flex-direction: column;
  transition: all 0.3s;
}
.matrix-card.active-card {
  border-color: rgba(59, 130, 246, 0.4);
  cursor: pointer;
  background: linear-gradient(180deg, rgba(30, 58, 138, 0.2) 0%, rgba(15, 23, 42, 0.6) 100%);
}
.matrix-card.active-card:hover {
  transform: translateY(-4px);
  border-color: #3b82f6;
  box-shadow: 0 16px 40px rgba(37, 99, 235, 0.3);
}
.matrix-status-tag {
  align-self: flex-start;
  font-size: 11px;
  font-weight: 700;
  padding: 2px 8px;
  border-radius: 10px;
  margin-bottom: 16px;
}
.matrix-status-tag.active {
  background: rgba(16, 185, 129, 0.2);
  color: #34d399;
}
.matrix-status-tag.plan {
  background: rgba(148, 163, 184, 0.15);
  color: #94a3b8;
}
.matrix-icon {
  font-size: 32px;
  margin-bottom: 12px;
}
.matrix-title {
  font-size: 18px;
  font-weight: 800;
  color: #ffffff;
  margin: 0 0 4px 0;
}
.matrix-domain {
  font-size: 12px;
  color: #38bdf8;
  font-family: monospace;
  margin-bottom: 12px;
}
.matrix-desc {
  font-size: 13px;
  color: #94a3b8;
  line-height: 1.5;
  margin: 0 0 24px 0;
  flex: 1;
}
.matrix-cta {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  font-weight: 600;
  color: #60a5fa;
}
.matrix-cta.disabled {
  color: #64748b;
}

/* 底部大横幅 */
.bottom-banner-section {
  padding: 80px 0;
}
.banner-inner {
  background: linear-gradient(135deg, rgba(30, 58, 138, 0.4) 0%, rgba(15, 23, 42, 0.9) 100%);
  border: 1px solid rgba(59, 130, 246, 0.3);
  border-radius: 32px;
  padding: 60px 40px;
  text-align: center;
  box-shadow: 0 30px 80px rgba(0, 0, 0, 0.6);
}
.banner-badge {
  display: inline-block;
  font-size: 13px;
  color: #38bdf8;
  font-weight: 700;
  margin-bottom: 16px;
  letter-spacing: 1px;
}
.banner-title {
  font-size: 36px;
  font-weight: 800;
  color: #ffffff;
  margin: 0 0 16px 0;
}
.banner-subtitle {
  max-width: 620px;
  margin: 0 auto 36px auto;
  font-size: 16px;
  color: #94a3b8;
  line-height: 1.6;
}
.banner-buttons {
  display: flex;
  justify-content: center;
  gap: 16px;
}
.banner-primary-btn {
  display: flex;
  align-items: center;
  gap: 10px;
  background: linear-gradient(135deg, #2563eb, #1d4ed8);
  color: #ffffff;
  border: none;
  padding: 14px 32px;
  border-radius: 30px;
  font-size: 15px;
  font-weight: 700;
  cursor: pointer;
  box-shadow: 0 10px 30px rgba(37, 99, 235, 0.4);
  transition: all 0.2s;
}
.banner-primary-btn:hover {
  transform: translateY(-2px);
  box-shadow: 0 14px 40px rgba(37, 99, 235, 0.6);
}
.banner-ghost-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.15);
  color: #ffffff;
  padding: 14px 24px;
  border-radius: 30px;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
}
.banner-ghost-btn:hover {
  background: rgba(255, 255, 255, 0.14);
}

/* Apple 风格页脚 */
.portal-footer {
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  background: #030408;
  padding: 60px 0 30px 0;
  font-size: 13px;
}
.footer-inner {
  max-width: 1280px;
  margin: 0 auto;
  padding: 0 24px;
}
.footer-top-row {
  display: grid;
  grid-template-columns: 2fr 1fr 1fr 1fr;
  gap: 40px;
  margin-bottom: 50px;
}
.f-brand {
  font-size: 20px;
  font-weight: 800;
  color: #ffffff;
  margin-bottom: 12px;
}
.f-dot { color: #38bdf8; }
.f-desc {
  font-size: 13px;
  line-height: 1.6;
  color: #64748b;
  max-width: 320px;
}
.f-col-title {
  font-size: 13px;
  font-weight: 700;
  color: #cbd5e1;
  margin-bottom: 16px;
}
.footer-links-col a {
  display: block;
  color: #94a3b8;
  text-decoration: none;
  margin-bottom: 10px;
  transition: color 0.2s;
}
.footer-links-col a:hover {
  color: #ffffff;
}
.footer-bottom-bar {
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  padding-top: 24px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  color: #64748b;
  font-size: 12px;
}
.footer-badges {
  display: flex;
  gap: 8px;
}

/* 动画 */
@keyframes pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.6; transform: scale(0.9); }
}
@keyframes glare {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(100%); }
}
.dropdown-fade-enter-active, .dropdown-fade-leave-active {
  transition: all 0.2s ease;
}
.dropdown-fade-enter-from, .dropdown-fade-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}

/* 响应式适配 */
@media (max-width: 1024px) {
  .hero-title { font-size: 44px; }
  .dashboard-glance-grid { grid-template-columns: repeat(2, 1fr); }
  .stream-console-simulation { grid-template-columns: 1fr; }
  .bento-grid { grid-template-columns: 1fr; }
  .bento-card-large { grid-column: span 1; }
  .matrix-grid { grid-template-columns: repeat(2, 1fr); }
  .footer-top-row { grid-template-columns: 1fr 1fr; }
}
@media (max-width: 640px) {
  .nav-links, .btn-search-trigger { display: none; }
  .hero-title { font-size: 32px; }
  .hero-subtitle { font-size: 15px; }
  .nodes-display-grid { grid-template-columns: 1fr; }
  .matrix-grid { grid-template-columns: 1fr; }
  .footer-top-row { grid-template-columns: 1fr; }
  .footer-bottom-bar { flex-direction: column; gap: 12px; }
}
</style>
