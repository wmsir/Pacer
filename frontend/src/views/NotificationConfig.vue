<template>
  <div class="notification-config-container page-container">
    <!-- 顶栏标题与操作区 -->
    <div class="page-header-flex">
      <div>
        <h2 class="page-title">
          <span>🔔 消息通知与告警通道配置中心</span>
        </h2>
        <div class="page-subtitle">
          构建协作即时通讯、移动短信服务、紧急语音电话三大梯队分类提醒体系，支持阿里巴巴与腾讯云多厂商配置及场景策略联动
        </div>
      </div>
      <div class="header-actions">
        <el-button type="primary" :icon="Check" :loading="saving" @click="handleSaveConfig">
          保存全部配置
        </el-button>
        <el-button :icon="Refresh" :loading="loading" @click="fetchConfig">
          重置 / 刷新
        </el-button>
      </div>
    </div>

    <!-- 顶部三大提醒方式分类卡片矩阵 -->
    <div class="category-summary-section">
      <el-row :gutter="16" class="category-cards-row">
        <!-- 类别 1: 协作即时通讯类 (IM & Bot) -->
        <el-col :xs="24" :sm="8">
          <div
            class="category-card im-card"
            :class="{ 'is-active': activeTab === 'wechat' || activeTab === 'dingtalk' || activeTab === 'feishu' }"
            @click="activeTab = 'wechat'"
          >
            <div class="category-head">
              <div class="category-badge im-badge">
                <span class="badge-icon">💬</span>
                <span class="badge-text">1. 协作即时通讯类 (IM)</span>
              </div>
              <el-tag size="small" type="success" effect="light">日常审批流转</el-tag>
            </div>
            <div class="category-desc">企业微信工作通知、阿里钉钉群机器人、字节飞书交互卡片</div>
            <div class="category-sub-channels">
              <span class="sub-pill" :class="{ on: form.wechat?.enabled }">
                企微: {{ form.wechat?.enabled ? '启用' : '关闭' }}
              </span>
              <span class="sub-pill" :class="{ on: form.dingtalk?.enabled }">
                钉钉: {{ form.dingtalk?.enabled ? '启用' : '关闭' }}
              </span>
              <span class="sub-pill" :class="{ on: form.feishu?.enabled }">
                飞书: {{ form.feishu?.enabled ? '启用' : '关闭' }}
              </span>
            </div>
            <div class="category-footer">
              <span class="cat-hint">工单提交、各级审批通过、流式下发日常协同推送</span>
            </div>
          </div>
        </el-col>

        <!-- 类别 2: 移动短信服务类 (SMS Gateway) -->
        <el-col :xs="24" :sm="8">
          <div
            class="category-card sms-card"
            :class="{ 'is-active': activeTab === 'sms' }"
            @click="activeTab = 'sms'"
          >
            <div class="category-head">
              <div class="category-badge sms-badge">
                <span class="badge-icon">📱</span>
                <span class="badge-text">2. 移动短信服务类 (SMS)</span>
              </div>
              <el-tag size="small" :type="form.sms?.enabled ? 'warning' : 'info'" effect="light">
                {{ form.sms?.enabled ? '已启用' : '未开启' }}
              </el-tag>
            </div>
            <div class="category-desc">多厂商短信服务接入：阿里巴巴(阿里云短信)、腾讯云短信、华为云与自建网关</div>
            <div class="category-sub-channels">
              <span class="vendor-tag">当前厂商: <b>{{ formatSmsProvider(form.sms?.provider) }}</b></span>
              <span class="sign-tag" v-if="form.sms?.signName">签名: {{ form.sms?.signName }}</span>
            </div>
            <div class="category-footer">
              <span class="cat-hint">加急催办、待办超时、变更临近离线强提醒</span>
            </div>
          </div>
        </el-col>

        <!-- 类别 3: 紧急电话语音类 (Voice Call) -->
        <el-col :xs="24" :sm="8">
          <div
            class="category-card voice-card"
            :class="{ 'is-active': activeTab === 'voiceCall' }"
            @click="activeTab = 'voiceCall'"
          >
            <div class="category-head">
              <div class="category-badge voice-badge">
                <span class="badge-icon">📞</span>
                <span class="badge-text">3. 紧急电话语音类 (Voice)</span>
              </div>
              <el-tag size="small" :type="form.voiceCall?.enabled ? 'danger' : 'info'" effect="light">
                {{ form.voiceCall?.enabled ? '已启用' : '未开启' }}
              </el-tag>
            </div>
            <div class="category-desc">阿里云语音 (SingleCallByTts)、腾讯云语音外呼、自建 SIP 网关直拨</div>
            <div class="category-sub-channels">
              <span class="vendor-tag">服务商: <b>{{ form.voiceCall?.provider || 'ALIYUN' }}</b></span>
              <span class="sign-tag">触发级别: P0 核心故障</span>
            </div>
            <div class="category-footer">
              <span class="cat-hint">生产库宕机、P0 变更失败、夜间紧急电话秒级唤醒</span>
            </div>
          </div>
        </el-col>
      </el-row>
    </div>

    <!-- 主配置面板 -->
    <el-tabs v-model="activeTab" type="border-card" class="config-tabs">
      <!-- 1. 企业微信配置 -->
      <el-tab-pane name="wechat">
        <template #label>
          <span class="tab-label">💬 🟢 企业微信工作通知 (WeChat Work)</span>
        </template>

        <div class="tab-content-grid">
          <div class="form-section">
            <el-form :model="form.wechat" label-position="top" label-width="140px">
              <el-form-item label="通道启用状态">
                <el-switch v-model="form.wechat.enabled" active-text="开启企微工作消息下发" inactive-text="关闭" />
              </el-form-item>

              <el-form-item label="集成协议模式">
                <el-radio-group v-model="form.wechat.mode">
                  <el-radio-button value="WEBSERVICE">WebService (WSDL / SOAP 接口)</el-radio-button>
                  <el-radio-button value="WEBHOOK">群机器人 Webhook</el-radio-button>
                </el-radio-group>
              </el-form-item>

              <template v-if="form.wechat.mode === 'WEBSERVICE'">
                <el-form-item label="WebService WSDL 地址">
                  <el-input
                    v-model="form.wechat.wsdlEndpoint"
                    placeholder="http://9.0.17.52:8083/wechat-wbs/services/ExternalDeptMessageService?wsdl"
                    clearable
                  />
                  <div class="form-tip">支持 WSDL 描述地址或 SOAP 服务端点 URL（系统会自动去除后缀兼容调用）</div>
                </el-form-item>

                <el-row :gutter="16">
                  <el-col :span="8">
                    <el-form-item label="系统接入标识 (sys_id)">
                      <el-input v-model="form.wechat.sysId" placeholder="例如 WMDB_SYSTEM" clearable />
                    </el-form-item>
                  </el-col>
                  <el-col :span="8">
                    <el-form-item label="系统认证密码 (sys_id_pass)">
                      <el-input v-model="form.wechat.sysIdPass" show-password placeholder="请输入对接密码" clearable />
                    </el-form-item>
                  </el-col>
                  <el-col :span="8">
                    <el-form-item label="系统标志 (sys_flag)">
                      <el-input v-model="form.wechat.sysFlag" placeholder="例如 1" clearable />
                    </el-form-item>
                  </el-col>
                </el-row>
              </template>

              <template v-else>
                <el-form-item label="企业微信群机器人 Webhook 地址">
                  <el-input
                    v-model="form.wechat.webhookUrl"
                    placeholder="https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=..."
                    clearable
                  />
                </el-form-item>
              </template>

              <el-row :gutter="16">
                <el-col :span="12">
                  <el-form-item label="单用户推送频次限流 (条/分钟)">
                    <el-input-number v-model="form.wechat.frequencyLimit" :min="0" :max="300" style="width: 100%;" />
                    <div class="form-tip">限制对同一 ERP 账号每分钟最多下发的消息数，0 表示不限流</div>
                  </el-form-item>
                </el-col>
                <el-col :span="12">
                  <el-form-item label="网络异常重试次数">
                    <el-input-number v-model="form.wechat.retryTimes" :min="0" :max="5" style="width: 100%;" />
                  </el-form-item>
                </el-col>
              </el-row>
            </el-form>
          </div>

          <!-- 在线测试诊断控制台 -->
          <div class="test-panel-card">
            <div class="test-panel-title">
              <span>🧪 企业微信通道在线联调测试</span>
            </div>
            <div class="test-panel-desc">
              输入接收人的企业微信 ERP 账号或手机号，快速验证 WebService 连通性。
            </div>

            <div class="test-field-group">
              <div class="field-label">测试接收人工号 / ERP：</div>
              <el-input v-model="wechatTestErp" placeholder="例如 zhangsan 或 102488" clearable>
                <template #prepend>👤 接收人</template>
              </el-input>
            </div>

            <div class="test-field-group">
              <div class="field-label">测试消息正文：</div>
              <el-input
                v-model="wechatTestMsg"
                type="textarea"
                :rows="3"
                placeholder="请输入要推送的测试内容"
              />
            </div>

            <div class="test-actions">
              <el-button
                type="success"
                :icon="Promotion"
                :loading="testingWechat"
                style="width: 100%;"
                @click="handleTestChannel('WECHAT')"
              >
                发送企微测试消息
              </el-button>
            </div>

            <div v-if="testResultMap.WECHAT" class="test-feedback-box" :class="{ 'is-ok': testResultMap.WECHAT.success, 'is-fail': !testResultMap.WECHAT.success }">
              <div class="feedback-status">
                <el-icon v-if="testResultMap.WECHAT.success" color="#67C23A"><CircleCheckFilled /></el-icon>
                <el-icon v-else color="#F56C6C"><CircleCloseFilled /></el-icon>
                <span>{{ testResultMap.WECHAT.success ? '✅ 推送成功' : '❌ 推送失败' }}</span>
                <span class="latency-tag" v-if="testResultMap.WECHAT.latencyMs">耗时: {{ testResultMap.WECHAT.latencyMs }}ms</span>
              </div>
              <div class="feedback-msg">{{ testResultMap.WECHAT.message }}</div>
            </div>
          </div>
        </div>
      </el-tab-pane>

      <!-- 2. 阿里钉钉群机器人配置 -->
      <el-tab-pane name="dingtalk">
        <template #label>
          <span class="tab-label">💬 🔵 阿里钉钉群机器人 (DingTalk)</span>
        </template>

        <div class="tab-content-grid">
          <div class="form-section">
            <el-form :model="form.dingtalk" label-position="top" label-width="140px">
              <el-form-item label="通道启用状态">
                <el-switch v-model="form.dingtalk.enabled" active-text="开启钉钉群机器人推送" inactive-text="关闭" />
              </el-form-item>

              <el-form-item label="钉钉自定义机器人 Webhook URL">
                <el-input
                  v-model="form.dingtalk.webhookUrl"
                  placeholder="https://oapi.dingtalk.com/robot/send?access_token=..."
                  clearable
                />
              </el-form-item>

              <el-form-item label="安全设置加签秘钥 (Secret)">
                <el-input
                  v-model="form.dingtalk.secret"
                  show-password
                  placeholder="SEC开头的安全密钥 (选填，若配置加签则必须填写)"
                  clearable
                />
                <div class="form-tip">支持 HMAC-SHA256 加签签名安全校验，防止 Webhook 泄露被恶意调用</div>
              </el-form-item>

              <el-row :gutter="16">
                <el-col :span="12">
                  <el-form-item label="通知时是否 @所有人">
                    <el-switch v-model="form.dingtalk.atAll" />
                  </el-form-item>
                </el-col>
                <el-col :span="12">
                  <el-form-item label="单群频次限制 (条/分钟)">
                    <el-input-number v-model="form.dingtalk.frequencyLimit" :min="1" :max="300" style="width: 100%;" />
                  </el-form-item>
                </el-col>
              </el-row>
            </el-form>
          </div>

          <div class="test-panel-card">
            <div class="test-panel-title">
              <span>🧪 钉钉机器人在线测试</span>
            </div>
            <div class="test-panel-desc">向钉钉群立即发送一条 Markdown 格式的数据库治理告警测试卡片。</div>

            <div class="test-field-group">
              <div class="field-label">测试消息正文：</div>
              <el-input v-model="dingtalkTestMsg" type="textarea" :rows="3" />
            </div>

            <div class="test-actions">
              <el-button
                type="primary"
                :icon="Promotion"
                :loading="testingDingtalk"
                style="width: 100%;"
                @click="handleTestChannel('DINGTALK')"
              >
                发送钉钉测试消息
              </el-button>
            </div>

            <div v-if="testResultMap.DINGTALK" class="test-feedback-box" :class="{ 'is-ok': testResultMap.DINGTALK.success, 'is-fail': !testResultMap.DINGTALK.success }">
              <div class="feedback-status">
                <el-icon v-if="testResultMap.DINGTALK.success" color="#67C23A"><CircleCheckFilled /></el-icon>
                <el-icon v-else color="#F56C6C"><CircleCloseFilled /></el-icon>
                <span>{{ testResultMap.DINGTALK.success ? '✅ 发送成功' : '❌ 发送失败' }}</span>
                <span class="latency-tag" v-if="testResultMap.DINGTALK.latencyMs">耗时: {{ testResultMap.DINGTALK.latencyMs }}ms</span>
              </div>
              <div class="feedback-msg">{{ testResultMap.DINGTALK.message }}</div>
            </div>
          </div>
        </div>
      </el-tab-pane>

      <!-- 3. 字节跳动飞书 / Lark 配置 -->
      <el-tab-pane name="feishu">
        <template #label>
          <span class="tab-label">💬 🟣 字节飞书 / Lark (Feishu)</span>
        </template>

        <div class="tab-content-grid">
          <div class="form-section">
            <el-form :model="form.feishu" label-position="top" label-width="140px">
              <el-form-item label="通道启用状态">
                <el-switch v-model="form.feishu.enabled" active-text="开启飞书互动卡片下发" inactive-text="关闭" />
              </el-form-item>

              <el-form-item label="飞书自定义群机器人 Webhook URL">
                <el-input
                  v-model="form.feishu.webhookUrl"
                  placeholder="https://open.feishu.cn/open-apis/bot/v2/hook/..."
                  clearable
                />
              </el-form-item>

              <el-form-item label="安全加签 Secret (加签密钥)">
                <el-input
                  v-model="form.feishu.secret"
                  show-password
                  placeholder="飞书加签校验签名密钥 (选填)"
                  clearable
                />
              </el-form-item>

              <el-form-item label="单群频次限流 (条/分钟)">
                <el-input-number v-model="form.feishu.frequencyLimit" :min="1" :max="300" style="width: 240px;" />
              </el-form-item>
            </el-form>
          </div>

          <div class="test-panel-card">
            <div class="test-panel-title">
              <span>🧪 飞书交互卡片在线测试</span>
            </div>
            <div class="test-panel-desc">向飞书群下发一条彩色标题、带跳转按钮的 Interactive 流程卡片。</div>

            <div class="test-field-group">
              <div class="field-label">测试卡片正文：</div>
              <el-input v-model="feishuTestMsg" type="textarea" :rows="3" />
            </div>

            <div class="test-actions">
              <el-button
                type="warning"
                :icon="Promotion"
                :loading="testingFeishu"
                style="width: 100%;"
                @click="handleTestChannel('FEISHU')"
              >
                发送飞书卡片测试
              </el-button>
            </div>

            <div v-if="testResultMap.FEISHU" class="test-feedback-box" :class="{ 'is-ok': testResultMap.FEISHU.success, 'is-fail': !testResultMap.FEISHU.success }">
              <div class="feedback-status">
                <el-icon v-if="testResultMap.FEISHU.success" color="#67C23A"><CircleCheckFilled /></el-icon>
                <el-icon v-else color="#F56C6C"><CircleCloseFilled /></el-icon>
                <span>{{ testResultMap.FEISHU.success ? '✅ 发送成功' : '❌ 发送失败' }}</span>
                <span class="latency-tag" v-if="testResultMap.FEISHU.latencyMs">耗时: {{ testResultMap.FEISHU.latencyMs }}ms</span>
              </div>
              <div class="feedback-msg">{{ testResultMap.FEISHU.message }}</div>
            </div>
          </div>
        </div>
      </el-tab-pane>

      <!-- 4. 移动短信服务 (SMS) —— [阿里云/腾讯云等多厂商] -->
      <el-tab-pane name="sms">
        <template #label>
          <span class="tab-label">📱 移动短信服务 (SMS Gateway)</span>
        </template>

        <div class="tab-content-grid">
          <div class="form-section">
            <el-alert
              title="短信通道定位：主要用于审批人员加急催办、待办超时离线提醒、工单重大异常通知。支持阿里云、腾讯云等厂商无缝切换。"
              type="info"
              :closable="false"
              show-icon
              style="margin-bottom: 16px;"
            />

            <el-form :model="form.sms" label-position="top" label-width="150px">
              <el-form-item label="短信服务启用状态">
                <el-switch v-model="form.sms.enabled" active-text="开启移动短信通知下发" inactive-text="关闭" />
              </el-form-item>

              <el-form-item label="选择短信服务提供商">
                <el-radio-group v-model="form.sms.provider" size="default">
                  <el-radio-button value="ALIYUN">
                    <span style="font-weight: 600;">🟧 阿里巴巴 (阿里云短信 Dysmsapi)</span>
                  </el-radio-button>
                  <el-radio-button value="TENCENT">
                    <span style="font-weight: 600;">🟦 腾讯云短信 (Tencent SMS)</span>
                  </el-radio-button>
                  <el-radio-button value="HUAWEI">
                    <span>🔴 华为云短信</span>
                  </el-radio-button>
                  <el-radio-button value="CUSTOM_HTTP">
                    <span>⚙️ 自建 HTTP 短信网关</span>
                  </el-radio-button>
                </el-radio-group>
              </el-form-item>

              <el-row :gutter="16">
                <el-col :span="12">
                  <el-form-item label="报备短信签名 (SignName)">
                    <el-input v-model="form.sms.signName" placeholder="例如 稳骥云平台、企业数据中心" clearable />
                    <div class="form-tip">必须与云厂商控制台审核通过的签名完全一致</div>
                  </el-form-item>
                </el-col>
                <el-col :span="12">
                  <el-form-item label="通用工单催办模版 Code / ID">
                    <el-input v-model="form.sms.templateCode" placeholder="例如 SMS_283910243 或 102456" clearable />
                    <div class="form-tip">云厂商审核通过的通知模板标识</div>
                  </el-form-item>
                </el-col>
              </el-row>

              <!-- 阿里云短信专属参数配置 -->
              <div v-if="form.sms.provider === 'ALIYUN'" class="provider-config-box aliyun-box">
                <div class="provider-box-title">🟧 阿里云短信服务参数设置 (Aliyun Dysmsapi)</div>
                <el-row :gutter="16">
                  <el-col :span="12">
                    <el-form-item label="AccessKey ID">
                      <el-input v-model="form.sms.aliyunAccessKeyId" placeholder="LTAI5t..." clearable />
                    </el-form-item>
                  </el-col>
                  <el-col :span="12">
                    <el-form-item label="AccessKey Secret">
                      <el-input v-model="form.sms.aliyunAccessKeySecret" show-password placeholder="请输入阿里云 Secret" clearable />
                    </el-form-item>
                  </el-col>
                </el-row>
                <el-form-item label="地域 Region ID">
                  <el-input v-model="form.sms.aliyunRegionId" placeholder="默认 cn-hangzhou" style="max-width: 260px;" clearable />
                </el-form-item>
              </div>

              <!-- 腾讯云短信专属参数配置 -->
              <div v-else-if="form.sms.provider === 'TENCENT'" class="provider-config-box tencent-box">
                <div class="provider-box-title">🟦 腾讯云短信服务参数设置 (Tencent Cloud SMS)</div>
                <el-row :gutter="16">
                  <el-col :span="12">
                    <el-form-item label="SecretId">
                      <el-input v-model="form.sms.tencentSecretId" placeholder="AKID..." clearable />
                    </el-form-item>
                  </el-col>
                  <el-col :span="12">
                    <el-form-item label="SecretKey">
                      <el-input v-model="form.sms.tencentSecretKey" show-password placeholder="请输入腾讯云 SecretKey" clearable />
                    </el-form-item>
                  </el-col>
                </el-row>
                <el-row :gutter="16">
                  <el-col :span="12">
                    <el-form-item label="短信 SDKAppID">
                      <el-input v-model="form.sms.tencentSdkAppId" placeholder="例如 1400888888" clearable />
                    </el-form-item>
                  </el-col>
                  <el-col :span="12">
                    <el-form-item label="地域 Region">
                      <el-input v-model="form.sms.tencentRegion" placeholder="默认 ap-guangzhou" clearable />
                    </el-form-item>
                  </el-col>
                </el-row>
              </div>

              <!-- 华为云或自建网关 -->
              <div v-else class="provider-config-box custom-box">
                <div class="provider-box-title">⚙️ 自建 HTTP / 华为云短信网关配置</div>
                <el-form-item label="网关 API 端点 URL">
                  <el-input v-model="form.sms.customApiEndpoint" placeholder="https://sms.yourdomain.com/v1/send" clearable />
                </el-form-item>
                <el-form-item label="网关认证 Token / API Key">
                  <el-input v-model="form.sms.customApiKey" show-password placeholder="Authorization Token" clearable />
                </el-form-item>
              </div>

              <!-- 防刷与限额保护 -->
              <el-row :gutter="16" style="margin-top: 14px;">
                <el-col :span="12">
                  <el-form-item label="单人单日短信限额 (条/人/天)">
                    <el-input-number v-model="form.sms.dailyLimitPerUser" :min="1" :max="100" style="width: 100%;" />
                    <div class="form-tip">防止测试或误发产生资费消耗，超出后当日自动熔断保护</div>
                  </el-form-item>
                </el-col>
                <el-col :span="12">
                  <el-form-item label="异常失败自动重试次数">
                    <el-input-number v-model="form.sms.retryTimes" :min="0" :max="5" style="width: 100%;" />
                  </el-form-item>
                </el-col>
              </el-row>
            </el-form>
          </div>

          <!-- 在线短信测试控制台 -->
          <div class="test-panel-card">
            <div class="test-panel-title">
              <span>🧪 移动短信通道在线下发测试 (Test SMS)</span>
            </div>
            <div class="test-panel-desc">
              输入接收人手机号码，针对当前配置的服务商（阿里巴巴 / 腾讯云等）发起一条实时代发测试短信。
            </div>

            <div class="test-field-group">
              <div class="field-label">测试目标手机号码：</div>
              <el-input v-model="smsTestPhone" placeholder="请输入 11 位测试手机号" clearable>
                <template #prepend>📱 手机号</template>
              </el-input>
            </div>

            <div class="test-field-group">
              <div class="field-label">测试短信内容 / 模板变量：</div>
              <el-input
                v-model="smsTestMsg"
                type="textarea"
                :rows="3"
                placeholder="例如：您有工单 #1024 处于待审批状态，请及时处理。"
              />
            </div>

            <div class="test-actions">
              <el-button
                type="warning"
                :icon="Promotion"
                :loading="testingSms"
                style="width: 100%;"
                @click="handleTestChannel('SMS')"
              >
                发送短信测试 (当前厂商: {{ formatSmsProvider(form.sms?.provider) }})
              </el-button>
            </div>

            <div v-if="testResultMap.SMS" class="test-feedback-box" :class="{ 'is-ok': testResultMap.SMS.success, 'is-fail': !testResultMap.SMS.success }">
              <div class="feedback-status">
                <el-icon v-if="testResultMap.SMS.success" color="#67C23A"><CircleCheckFilled /></el-icon>
                <el-icon v-else color="#F56C6C"><CircleCloseFilled /></el-icon>
                <span>{{ testResultMap.SMS.success ? '✅ 短信下发成功' : '❌ 短信发送失败' }}</span>
                <span class="latency-tag" v-if="testResultMap.SMS.latencyMs">耗时: {{ testResultMap.SMS.latencyMs }}ms</span>
              </div>
              <div class="feedback-msg">{{ testResultMap.SMS.message }}</div>
              <div class="feedback-raw" v-if="testResultMap.SMS.rawResponse">
                <div class="raw-title">服务商回执报文:</div>
                <pre>{{ testResultMap.SMS.rawResponse }}</pre>
              </div>
            </div>
          </div>
        </div>
      </el-tab-pane>

      <!-- 5. 紧急电话语音外呼配置 -->
      <el-tab-pane name="voiceCall">
        <template #label>
          <span class="tab-label">📞 紧急电话智能语音外呼 (Voice Call)</span>
        </template>

        <div class="tab-content-grid">
          <div class="form-section">
            <el-alert
              title="电话外呼功能定位：用于系统发生 P0 级别严重变更故障、数据库宕机或高危恶意 SQL 阻断时，直接向值班 DBA 与主管发起实时电话呼叫！"
              type="warning"
              :closable="false"
              show-icon
              style="margin-bottom: 16px;"
            />

            <el-form :model="form.voiceCall" label-position="top" label-width="140px">
              <el-form-item label="语音外呼功能启用">
                <el-switch v-model="form.voiceCall.enabled" active-text="开启电话语音外呼" inactive-text="关闭" />
              </el-form-item>

              <el-form-item label="语音外呼服务提供商">
                <el-radio-group v-model="form.voiceCall.provider">
                  <el-radio-button value="ALIYUN">阿里云语音服务 (SingleCallByTts)</el-radio-button>
                  <el-radio-button value="TENCENT">腾讯云语音通知</el-radio-button>
                  <el-radio-button value="SIP_GATEWAY">自建企业 SIP / IVR 语音网关</el-radio-button>
                </el-radio-group>
              </el-form-item>

              <el-row :gutter="16">
                <el-col :span="12">
                  <el-form-item label="AccessKey ID / API Key">
                    <el-input v-model="form.voiceCall.accessKeyId" placeholder="LTAI..." clearable />
                  </el-form-item>
                </el-col>
                <el-col :span="12">
                  <el-form-item label="AccessKey Secret">
                    <el-input v-model="form.voiceCall.accessKeySecret" show-password placeholder="请输入语音密钥" clearable />
                  </el-form-item>
                </el-col>
              </el-row>

              <el-row :gutter="16">
                <el-col :span="12">
                  <el-form-item label="TTS 语音播报模板 Code">
                    <el-input v-model="form.voiceCall.templateCode" placeholder="例如 TTS_123456789" clearable />
                  </el-form-item>
                </el-col>
                <el-col :span="12">
                  <el-form-item label="外呼主叫显示号码">
                    <el-input v-model="form.voiceCall.calledShowNumber" placeholder="例如 057188888888" clearable />
                  </el-form-item>
                </el-col>
              </el-row>

              <el-form-item label="默认紧急值班联系人手机号列表">
                <div v-for="(_, index) in form.voiceCall.emergencyContacts" :key="index" style="display: flex; gap: 8px; margin-bottom: 8px;">
                  <el-input v-model="form.voiceCall.emergencyContacts[index]" placeholder="请输入 11 位手机号码" style="max-width: 320px;" />
                  <el-button type="danger" plain :icon="Delete" @click="removeContact(Number(index))">删除</el-button>
                </div>
                <el-button type="primary" plain :icon="Plus" size="small" @click="addContact">添加紧急联系人号码</el-button>
              </el-form-item>
            </el-form>
          </div>

          <!-- 在线测试语音外呼 -->
          <div class="test-panel-card">
            <div class="test-panel-title">
              <span>🧪 电话语音外呼在线拨测 (Test Voice Call)</span>
            </div>
            <div class="test-panel-desc">
              输入被叫手机号码，发起一次自动化告警语音呼叫握手测试。
            </div>

            <div class="test-field-group">
              <div class="field-label">测试被叫手机号：</div>
              <el-input
                v-model="voiceTestPhone"
                placeholder="请输入 11 位手机号码 (如 13800138000)"
                clearable
              >
                <template #prepend>📱 手机号</template>
              </el-input>
            </div>

            <div class="test-actions" style="margin-top: 16px;">
              <el-button
                type="danger"
                :icon="Phone"
                :loading="testingVoice"
                style="width: 100%;"
                @click="handleTestChannel('VOICE_CALL')"
              >
                发起语音外呼拨测测试
              </el-button>
            </div>

            <div v-if="testResultMap.VOICE_CALL" class="test-feedback-box" :class="{ 'is-ok': testResultMap.VOICE_CALL.success, 'is-fail': !testResultMap.VOICE_CALL.success }">
              <div class="feedback-status">
                <el-icon v-if="testResultMap.VOICE_CALL.success" color="#67C23A"><CircleCheckFilled /></el-icon>
                <el-icon v-else color="#F56C6C"><CircleCloseFilled /></el-icon>
                <span>{{ testResultMap.VOICE_CALL.success ? '✅ 外呼握手成功' : '❌ 外呼失败' }}</span>
                <span class="latency-tag" v-if="testResultMap.VOICE_CALL.latencyMs">耗时: {{ testResultMap.VOICE_CALL.latencyMs }}ms</span>
              </div>
              <div class="feedback-msg">{{ testResultMap.VOICE_CALL.message }}</div>
            </div>
          </div>
        </div>
      </el-tab-pane>

      <!-- 6. 提醒方式分类使用与告警策略矩阵 (Policy & Usage Matrix) -->
      <el-tab-pane name="policy">
        <template #label>
          <span class="tab-label">⚙️ 提醒方式分类使用与场景联动矩阵 (Policies)</span>
        </template>

        <div style="max-width: 900px; padding: 12px 0;">
          <el-alert
            title="三大提醒方式分类使用原则：日常流转走即时通讯，加急催办增补短信，P0 故障直呼语音电话。"
            type="success"
            :closable="false"
            show-icon
            style="margin-bottom: 18px;"
          />

          <!-- 场景使用联动矩阵表格 -->
          <div class="matrix-title">1. 业务场景与三大提醒方式联动矩阵</div>
          <table class="policy-matrix-table">
            <thead>
              <tr>
                <th style="width: 240px;">业务事件 / 触发场景</th>
                <th style="width: 160px;">💬 即时通讯类</th>
                <th style="width: 160px;">📱 移动短信类</th>
                <th style="width: 160px;">📞 电话语音类</th>
                <th>使用说明</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td><b>日常工单提交流转 & 审批通过</b></td>
                <td>
                  <el-switch v-model="form.policy.dailyNotifyUseIm" active-text="启用" />
                </td>
                <td>
                  <span class="tag-subtle">按需可选</span>
                </td>
                <td>
                  <span class="tag-disabled">关闭</span>
                </td>
                <td class="desc-cell">工作消息富文本推送，低成本、低打扰</td>
              </tr>
              <tr>
                <td><b>工单加急催办 / 待办超时</b></td>
                <td>
                  <el-tag size="small" type="success">常驻开启</el-tag>
                </td>
                <td>
                  <el-switch v-model="form.policy.urgeNotifyUseSms" active-text="同步短信" />
                </td>
                <td>
                  <span class="tag-disabled">关闭</span>
                </td>
                <td class="desc-cell">双通道强提醒，防止审批延期影响生产上线</td>
              </tr>
              <tr>
                <td><b>SQL 变更执行失败 / 驳回提醒</b></td>
                <td>
                  <el-switch v-model="form.policy.notifyOnFailed" active-text="启用" />
                </td>
                <td>
                  <el-switch v-model="form.policy.failedNotifyUseSms" active-text="同步短信" />
                </td>
                <td>
                  <span class="tag-subtle">可选</span>
                </td>
                <td class="desc-cell">变更异常迅速告知申请人与审批主管</td>
              </tr>
              <tr class="highlight-p0-row">
                <td><b>🚨 P0 核心故障 / 生产库宕机 / 恶意拦截</b></td>
                <td>
                  <el-tag size="small" type="danger">紧急推送</el-tag>
                </td>
                <td>
                  <el-tag size="small" type="warning">强制短信</el-tag>
                </td>
                <td>
                  <el-switch v-model="form.policy.emergencyUseVoiceCall" active-text="电话直呼" />
                </td>
                <td class="desc-cell"><b>秒级呼叫值班 DBA 与安全负责人</b></td>
              </tr>
            </tbody>
          </table>

          <el-divider style="margin: 24px 0;" />

          <div class="matrix-title">2. 全局推送频次与夜间静默期</div>
          <el-form :model="form.policy" label-position="left" label-width="260px">
            <el-form-item label="开启夜间静默期控制">
              <el-switch v-model="form.policy.quietHoursEnabled" />
            </el-form-item>
            <el-form-item label="静默时段范围" v-if="form.policy.quietHoursEnabled">
              <div style="display: flex; gap: 8px; align-items: center;">
                <el-time-picker v-model="form.policy.quietHoursStart" value-format="HH:mm" placeholder="22:00" />
                <span>至</span>
                <el-time-picker v-model="form.policy.quietHoursEnd" value-format="HH:mm" placeholder="08:00" />
              </div>
              <div class="form-tip">在静默时段内，非 P0 紧急消息将暂缓下发，避免深夜打扰工作人员。</div>
            </el-form-item>
          </el-form>
        </div>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import {
  Check,
  Refresh,
  Promotion,
  Plus,
  Delete,
  Phone,
  CircleCheckFilled,
  CircleCloseFilled
} from '@element-plus/icons-vue'
import request from '../utils/request'

const activeTab = ref('wechat')
const loading = ref(false)
const saving = ref(false)

const testingWechat = ref(false)
const testingDingtalk = ref(false)
const testingFeishu = ref(false)
const testingSms = ref(false)
const testingVoice = ref(false)

const wechatTestErp = ref('zhangsan')
const wechatTestMsg = ref('【稳骥工单变更】您的 SQL 工单 #1024 已审批通过，预计将在维护窗口自动执行。')

const dingtalkTestMsg = ref('【稳骥告警通知】生产库 `rm-uf6ab...` 当前存在 2 个慢查询阻塞会话，请 DBA 及时关注。')
const feishuTestMsg = ref('【稳骥飞书通知】工单 #1024 预执行 Dry-Run 校验通过，影响行数：12 行。')

const smsTestPhone = ref('13800138000')
const smsTestMsg = ref('【稳骥云平台】工单 #1024 待开发组长初审，影响行数 12 行，请登录系统处理。')

const voiceTestPhone = ref('13800138000')

const testResultMap = reactive<Record<string, any>>({
  WECHAT: null,
  DINGTALK: null,
  FEISHU: null,
  SMS: null,
  VOICE_CALL: null
})

const form = reactive<any>({
  wechat: {
    enabled: true,
    mode: 'WEBSERVICE',
    wsdlEndpoint: 'http://9.0.17.52:8083/wechat-wbs/services/ExternalDeptMessageService?wsdl',
    sysId: 'WMDB_SYSTEM',
    sysIdPass: 'wmdb_pass_123',
    sysFlag: '1',
    webhookUrl: '',
    retryTimes: 3,
    frequencyLimit: 30
  },
  dingtalk: {
    enabled: true,
    webhookUrl: 'https://oapi.dingtalk.com/robot/send?access_token=your_token',
    secret: 'SEC_your_secret',
    atAll: false,
    frequencyLimit: 60
  },
  feishu: {
    enabled: true,
    mode: 'WEBHOOK',
    webhookUrl: 'https://open.feishu.cn/open-apis/bot/v2/hook/your_feishu_token',
    secret: '',
    appId: '',
    appSecret: '',
    frequencyLimit: 60
  },
  sms: {
    enabled: true,
    provider: 'ALIYUN',
    signName: '稳骥云平台',
    templateCode: 'SMS_283910243',
    aliyunAccessKeyId: 'LTAI5t_your_key',
    aliyunAccessKeySecret: 'your_secret',
    aliyunRegionId: 'cn-hangzhou',
    tencentSecretId: 'AKID_your_sid',
    tencentSecretKey: 'your_skey',
    tencentSdkAppId: '1400888888',
    tencentRegion: 'ap-guangzhou',
    customApiEndpoint: '',
    customApiKey: '',
    dailyLimitPerUser: 20,
    retryTimes: 2
  },
  voiceCall: {
    enabled: false,
    provider: 'ALIYUN',
    endpoint: 'dyvmsapi.aliyuncs.com',
    accessKeyId: 'LTAI_your_key',
    accessKeySecret: 'your_secret',
    templateCode: 'TTS_123456',
    calledShowNumber: '057188888888',
    emergencyContacts: ['13800138000'],
    triggerEvents: ['TICKET_FAILED_P0', 'INSTANCE_DOWN']
  },
  policy: {
    notifyOnSubmit: true,
    notifyOnAudited: true,
    notifyOnExecuted: true,
    notifyOnFailed: true,
    notifyOnRiskIntercept: true,
    quietHoursEnabled: false,
    quietHoursStart: '22:00',
    quietHoursEnd: '08:00',
    dailyNotifyUseIm: true,
    urgeNotifyUseSms: true,
    emergencyUseVoiceCall: true,
    failedNotifyUseSms: false
  }
})

const formatSmsProvider = (provider?: string) => {
  switch (provider) {
    case 'ALIYUN': return '阿里巴巴 (阿里云短信)'
    case 'TENCENT': return '腾讯云短信'
    case 'HUAWEI': return '华为云短信'
    case 'CUSTOM_HTTP': return '自建 HTTP 网关'
    default: return provider || '阿里巴巴 (阿里云短信)'
  }
}

const fetchConfig = async () => {
  loading.value = true
  try {
    const res: any = await request.get('/v1/notification/config')
    if (res.data) {
      if (res.data.wechat) Object.assign(form.wechat, res.data.wechat)
      if (res.data.dingtalk) Object.assign(form.dingtalk, res.data.dingtalk)
      if (res.data.feishu) Object.assign(form.feishu, res.data.feishu)
      if (res.data.sms) Object.assign(form.sms, res.data.sms)
      if (res.data.voiceCall) {
        Object.assign(form.voiceCall, res.data.voiceCall)
        if (!form.voiceCall.emergencyContacts) form.voiceCall.emergencyContacts = ['13800138000']
      }
      if (res.data.policy) Object.assign(form.policy, res.data.policy)
    }
  } catch (err: any) {
    ElMessage.error('加载通知通道配置失败: ' + (err.message || '未知错误'))
  } finally {
    loading.value = false
  }
}

const handleSaveConfig = async () => {
  saving.value = true
  try {
    await request.post('/v1/notification/config', form)
    ElMessage.success('消息通知与告警通道配置已保存并即时生效！')
    await fetchConfig()
  } catch (err: any) {
    ElMessage.error('保存失败: ' + (err.response?.data?.message || err.message))
  } finally {
    saving.value = false
  }
}

const handleTestChannel = async (channel: string) => {
  let target = ''
  let message = ''
  let provider = ''

  if (channel === 'WECHAT') {
    if (!wechatTestErp.value.trim()) {
      ElMessage.warning('请输入测试接收人 ERP 账号 / 工号')
      return
    }
    target = wechatTestErp.value.trim()
    message = wechatTestMsg.value
    testingWechat.value = true
  } else if (channel === 'DINGTALK') {
    message = dingtalkTestMsg.value
    testingDingtalk.value = true
  } else if (channel === 'FEISHU') {
    message = feishuTestMsg.value
    testingFeishu.value = true
  } else if (channel === 'SMS') {
    if (!smsTestPhone.value.trim()) {
      ElMessage.warning('请输入测试目标手机号码')
      return
    }
    target = smsTestPhone.value.trim()
    message = smsTestMsg.value
    provider = form.sms?.provider || 'ALIYUN'
    testingSms.value = true
  } else if (channel === 'VOICE_CALL') {
    if (!voiceTestPhone.value.trim()) {
      ElMessage.warning('请输入测试被叫手机号')
      return
    }
    target = voiceTestPhone.value.trim()
    testingVoice.value = true
  }

  try {
    const res: any = await request.post('/v1/notification/test-channel', {
      channel,
      target,
      message,
      provider
    })
    testResultMap[channel] = res.data
    if (res.data?.success) {
      ElMessage.success(res.data.message || '通道测试下发成功！')
    } else {
      ElMessage.error(res.data.message || '通道测试返回失败')
    }
  } catch (err: any) {
    ElMessage.error('测试请求异常: ' + (err.response?.data?.message || err.message))
  } finally {
    testingWechat.value = false
    testingDingtalk.value = false
    testingFeishu.value = false
    testingSms.value = false
    testingVoice.value = false
  }
}

const addContact = () => {
  if (!form.voiceCall.emergencyContacts) {
    form.voiceCall.emergencyContacts = []
  }
  form.voiceCall.emergencyContacts.push('')
}

const removeContact = (index: number) => {
  form.voiceCall.emergencyContacts.splice(index, 1)
}

onMounted(() => {
  fetchConfig()
})
</script>

<style scoped>
.notification-config-container {
  width: 100%;
  box-sizing: border-box;
}

.page-header-flex {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 18px;
  flex-wrap: wrap;
  gap: 12px;
}

/* 顶部三大提醒方式分类卡片 */
.category-summary-section {
  margin-bottom: 20px;
}

.category-card {
  background: #ffffff;
  border: 1px solid #e2e8f0;
  border-radius: 10px;
  padding: 14px 16px;
  cursor: pointer;
  transition: all 0.25s ease;
  display: flex;
  flex-direction: column;
  min-height: 125px;
}

.category-card:hover {
  border-color: #3b82f6;
  transform: translateY(-2px);
  box-shadow: 0 8px 18px -4px rgba(59, 130, 246, 0.1);
}

.category-card.is-active {
  border-color: #2563eb;
  background: #f8fafc;
  box-shadow: 0 0 0 1.5px #2563eb;
}

.category-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 6px;
}

.category-badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 14px;
  font-weight: 700;
  color: #0f172a;
}

.badge-icon {
  font-size: 16px;
}

.category-desc {
  font-size: 12px;
  color: #64748b;
  margin-bottom: 8px;
  line-height: 1.4;
}

.category-sub-channels {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
  margin-top: auto;
  margin-bottom: 6px;
}

.sub-pill {
  font-size: 11.5px;
  background: #f1f5f9;
  color: #64748b;
  padding: 1px 6px;
  border-radius: 4px;
}

.sub-pill.on {
  background: #ecfdf5;
  color: #059669;
  font-weight: 600;
}

.vendor-tag {
  font-size: 11.5px;
  color: #334155;
  background: #eff6ff;
  padding: 1px 6px;
  border-radius: 4px;
}

.sign-tag {
  font-size: 11.5px;
  color: #64748b;
}

.category-footer {
  border-top: 1px dashed #e2e8f0;
  padding-top: 6px;
  font-size: 11.5px;
  color: #94a3b8;
}

/* 主配置 Tabs */
.config-tabs {
  border-radius: 10px;
  background: #ffffff;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
}

.tab-label {
  font-size: 13.5px;
  font-weight: 600;
}

.tab-content-grid {
  display: grid;
  grid-template-columns: 1fr 420px;
  gap: 24px;
  padding: 14px 6px;
}

@media (max-width: 1100px) {
  .tab-content-grid {
    grid-template-columns: 1fr;
  }
}

.form-section {
  padding-right: 12px;
}

.form-tip {
  font-size: 12px;
  color: #64748b;
  margin-top: 4px;
  line-height: 1.4;
}

/* 云厂商参数专属外框 */
.provider-config-box {
  border-radius: 8px;
  padding: 14px;
  margin-top: 12px;
  border: 1px solid #e2e8f0;
}

.aliyun-box {
  background: #fffbf5;
  border-color: #fed7aa;
}

.tencent-box {
  background: #f0f9ff;
  border-color: #bae6fd;
}

.custom-box {
  background: #f8fafc;
  border-color: #cbd5e1;
}

.provider-box-title {
  font-size: 13.5px;
  font-weight: 700;
  color: #0f172a;
  margin-bottom: 12px;
}

/* 在线测试诊断卡片 */
.test-panel-card {
  background: #f8fafc;
  border: 1px solid #e2e8f0;
  border-radius: 10px;
  padding: 18px 20px;
  display: flex;
  flex-direction: column;
}

.test-panel-title {
  font-size: 14.5px;
  font-weight: 700;
  color: #0f172a;
  margin-bottom: 6px;
}

.test-panel-desc {
  font-size: 12.5px;
  color: #64748b;
  margin-bottom: 16px;
  line-height: 1.5;
}

.test-field-group {
  margin-bottom: 12px;
}

.field-label {
  font-size: 13px;
  font-weight: 600;
  color: #334155;
  margin-bottom: 6px;
}

.test-actions {
  margin-top: 12px;
}

.test-feedback-box {
  margin-top: 16px;
  border-radius: 6px;
  padding: 12px 14px;
  background: #ffffff;
  border: 1px solid #e2e8f0;
}

.test-feedback-box.is-ok {
  border-color: #67c23a;
  background: #f0f9eb;
}

.test-feedback-box.is-fail {
  border-color: #f56c6c;
  background: #fef0f0;
}

.feedback-status {
  display: flex;
  align-items: center;
  gap: 6px;
  font-weight: 700;
  font-size: 13.5px;
  margin-bottom: 6px;
}

.latency-tag {
  margin-left: auto;
  font-size: 11px;
  color: #64748b;
  font-weight: normal;
}

.feedback-msg {
  font-size: 12.5px;
  color: #334155;
  line-height: 1.5;
}

.feedback-raw {
  margin-top: 10px;
  border-top: 1px dashed #cbd5e1;
  padding-top: 8px;
}

.raw-title {
  font-size: 11px;
  color: #64748b;
  font-weight: 600;
}

.feedback-raw pre {
  margin: 4px 0 0 0;
  font-family: monospace;
  font-size: 11px;
  color: #475569;
  background: #ffffff;
  padding: 8px;
  border-radius: 4px;
  overflow-x: auto;
  max-height: 140px;
}

/* 策略矩阵表格样式 */
.matrix-title {
  font-size: 14px;
  font-weight: 700;
  color: #0f172a;
  margin-bottom: 10px;
}

.policy-matrix-table {
  width: 100%;
  border-collapse: collapse;
  margin-top: 8px;
  background: #ffffff;
  border-radius: 8px;
  overflow: hidden;
  border: 1px solid #e2e8f0;
}

.policy-matrix-table th,
.policy-matrix-table td {
  padding: 12px 14px;
  border: 1px solid #e2e8f0;
  font-size: 13px;
  text-align: left;
}

.policy-matrix-table th {
  background: #f8fafc;
  color: #475569;
  font-weight: 600;
}

.policy-matrix-table td {
  color: #1e293b;
}

.highlight-p0-row {
  background: #fff7ed;
}

.desc-cell {
  font-size: 12px;
  color: #64748b;
}

.tag-subtle {
  font-size: 12px;
  color: #94a3b8;
}

.tag-disabled {
  font-size: 12px;
  color: #cbd5e1;
}
</style>
