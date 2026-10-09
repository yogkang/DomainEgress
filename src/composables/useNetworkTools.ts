import { ref } from 'vue';
import { call, desktop, type NetworkTool, type NetworkProbeRequest, type NetworkProbeResult } from '../api';
import { Radio, Network, Globe2, Server, FileKey2, Wifi } from 'lucide-vue-next';
export function useNetworkTools({ notify }: {
    notify: (message: string, failed?: boolean) => void;
}) {
    const networkTool = ref<NetworkTool>("ping");
    const networkProbe = ref<NetworkProbeRequest>({
        tool: "ping",
        target: "example.com",
        port: null,
        timeout_ms: 3000,
        count: 4,
        method: "GET",
        headers: "",
        body: "",
        payload: "",
        payload_hex: false,
    });
    const networkResult = ref<NetworkProbeResult | null>(null), networkBusy = ref(false);
    const networkTools = [
        { id: "ping", title: "Ping", icon: Radio },
        { id: "telnet", title: "Telnet / TCP", icon: Server },
        { id: "certificate", title: "域名证书", icon: FileKey2 },
        { id: "traceroute", title: "Traceroute", icon: Network },
        { id: "tcp", title: "TCP 客户端", icon: Wifi },
        { id: "udp", title: "UDP 客户端", icon: Wifi },
        { id: "http", title: "HTTP 客户端", icon: Globe2 },
        { id: "websocket", title: "WebSocket", icon: Radio },
    ] as {
        id: NetworkTool;
        title: string;
        icon: typeof Radio;
    }[];
    function selectNetworkTool(tool: NetworkTool) {
        networkTool.value = tool;
        networkProbe.value.tool = tool;
        if (tool === "http" && !networkProbe.value.target.includes("://"))
            networkProbe.value.target = `https://${networkProbe.value.target}`;
        if (tool === "websocket" && !networkProbe.value.target.startsWith("ws"))
            networkProbe.value.target = `wss://${networkProbe.value.target}`;
    }
    function detailText(value: unknown) {
        return typeof value === "string" ? value : JSON.stringify(value, null, 2);
    }
    async function runNetworkProbe() {
        if (!desktop) {
            notify("请在 DomainEgress 桌面应用中执行真实网络探测", true);
            return;
        }
        networkBusy.value = true;
        networkResult.value = null;
        const request = { ...networkProbe.value, tool: networkTool.value };
        try {
            networkResult.value = await call<NetworkProbeResult>("run_network_probe", {
                request,
            });
        }
        catch (e) {
            notify(String(e), true);
        }
        finally {
            networkBusy.value = false;
        }
    }
    async function cancelNetworkProbe() {
        if (!desktop)
            return;
        try {
            await call("cancel_network_probe");
        }
        catch (e) {
            notify(String(e), true);
        }
    }
    return { networkTool, networkProbe, networkResult, networkBusy, networkTools, selectNetworkTool, detailText, runNetworkProbe, cancelNetworkProbe };
}
