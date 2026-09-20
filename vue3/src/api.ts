import { invoke, isTauri } from '@tauri-apps/api/core'
export interface Config {
  version: number; access_mode: 'whitelist' | 'blacklist'; whitelist: string[]; blacklist: string[];
  http_host: string; http_port: number; socks_host: string; socks_port: number; auto_start: boolean;
  log_level: string; log_retention_days: number; whitelist_added_at: Record<string, number>; blacklist_added_at: Record<string, number>;
  trend_retention_days: number; font_scale: number;
  ssh_profiles: SshProfile[]; active_ssh_profile: string | null; ssh_forwards: SshForwardRule[]; ssh_forward_options: string[]; icloud_sync_enabled: boolean;
  gist_provider: 'github' | 'gitee'; gist_id: string; gist_file_name: string;
}
export interface SshHop { host: string; port: number; username: string; auth: 'agent' | 'keychain' | 'password'; keychain_id?: string | null }
export interface SshProfile { id: string; name: string; hops: SshHop[]; enabled: boolean }
export interface SshForwardRule { id: string; name: string; project: string; note: string; local_port: number | null; remote_host: string; remote_port: number; bind_host: string; ssh_host: string; ssh_port: number; ssh_username: string; ssh_keychain_id?: string | null; auto_start: boolean; ssh_options: string[] }
export interface LogEntry { timestamp: number; level: string; source: string; method: string; target: string; params: string; outcome: string }
export interface NetworkInterface { name: string; kind: string; addresses: string[] }
export interface GeoLocation { country: string; country_code: string; region: string; city: string; isp: string; source: string; confidence: string }
export interface EgressAddress { ip: string; source: string; location: GeoLocation | null }
export interface EgressProbe { addresses: EgressAddress[]; confidence: string; error: string | null }
export interface PublicIpProbe { system: EgressProbe; ipv6: EgressProbe | null }
export interface UpdateInfo { current_version: string; latest_version: string | null; release_url: string | null; available: boolean; error: string | null }
export interface Snapshot { config: Config; running: boolean; logs: LogEntry[]; traffic: number[]; message: string | null; ssh_running?: boolean; ssh_local_port?: number | null; ssh_forward_ports?: Record<string, number>; interfaces?: NetworkInterface[]; hostname?: string }
export interface PortRow { port: string; pid: number; name: string; started: string; elapsed: string }
export interface CloudAccount { id: string; provider: 'aliyun'; display_name: string; auth_method: 'ram_access_key' | 'access_key'; access_key_hint: string; verified_account_id: string | null; verification_status: string; created_at: number; updated_at: number }
export interface SaveCloudAccountInput { display_name: string; auth_method: 'ram_access_key' | 'access_key'; access_key_id: string; access_key_secret: string }
export interface CloudRegion { id: string; name: string }
export interface CloudSecurityGroup { id: string; name: string; vpc_id: string; group_type: string }
export interface CloudSecurityRule { id: string; direction: string; protocol: string; port_range: string; priority: number; action: string; source_cidr: string; description: string; managed: boolean }
export interface ManagedRuleConfig { id: string; account_id: string; region: string; security_group_id: string; protocol: string; port_range: string; priority: number; description: string; enabled: boolean; last_source_cidr: string | null; last_synced_at: number | null; last_error: string | null }
export interface CreateManagedRuleInput { account_id: string; region: string; security_group_id: string; protocol: string; port_range: string; priority: number }
export interface ManagedRuleSyncResult { config: ManagedRuleConfig; action: string; rule_id: string }
export const desktop = isTauri()
export const defaults: Config = { version: 2, access_mode: 'whitelist', whitelist: [
  'agentclientprotocol.com','alicdn.com','aliyun.com','aliyuncs.com','apache.org','apipost.net','bcebos.com','bdstatic.com','brucege.com','byteacctimg.com','cloudfront.net','codebuddy.cn','corretto.aws','ctobsnssdk.com','getcomposer.org','gitee.com','github.com','github.io','githubcopilot.com','gist.github.com','gist.githubusercontent.com','githubusercontent.com','golang.org','google.cn','google.com','googleapis.com','googletagmanager.co','goproxy.cn','gradle.org','ibb.co','jboss.org','laravel-idea.com','lierda.com','marscode.cn','marscode.com','maven.org','mixpanel.com','mvnrepository.com','myqcloud.com','pypi.org','python.org','qcloudimg.com','qoder.com','qoder.com.cn','qodo.ai','rumt-zh.com','schemastore.org','senthink.com','sinfere.com','spring.io','tencent-cloud.com','tencent.com','trae.com.cn','zijieapi.com','api.app.prod.grazie.aws.intellij.net','api.jetbrains.ai','cache-redirector.jetbrains.com','cloudconfig.jetbrains.com','code-with-me.jetbrains.com','code-with-me.jetbrains.com.cn','download-alibaba.jetbrains.com.cn','download-cdn.clf.jetbrains.com.cn','download-cdn.jetbrains.com','download-cdn.jetbrains.com.cn','download.clf.jetbrains.com.cn','download.jetbrains.com','downloads.marketplace.jetbrains.com','eks.jetbrains.com.cn','frameworks.jetbrains.com','marketplace.jetbrains.com','marketplace.jetbrains.com.cn','oauth.account.jetbrains.com','plugins.jetbrains.com','redirector.jetbrains.com.cn','resources.jetbrains.com','resources.jetbrains.com.cn','vulnerability-search.jetbrains.com','www.jetbrains.com'
], blacklist: [], http_host: '127.0.0.1', http_port: 19876, socks_host: '127.0.0.1', socks_port: 16789, auto_start: false, log_level: 'info', log_retention_days: 30, trend_retention_days: 21, font_scale: 100, whitelist_added_at: {}, blacklist_added_at: {}, ssh_profiles: [], active_ssh_profile: null, ssh_forwards: [], ssh_forward_options: [], icloud_sync_enabled: false, gist_provider: 'github', gist_id: '', gist_file_name: 'domain-egress-rules.json' }
export async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!desktop) throw new Error('请在 DomainEgress 桌面应用中操作；浏览器仅提供界面预览。')
  return invoke<T>(command, args)
}
