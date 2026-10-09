import { inject, type InjectionKey } from 'vue';
import type { useWorkspace } from './useWorkspace';
export const workspaceKey: InjectionKey<ReturnType<typeof useWorkspace>> = Symbol('DomainEgress workspace');
export function useWorkspaceContext() {
    const workspace = inject(workspaceKey);
    if (!workspace)
        throw new Error('Workspace provider is missing');
    return workspace;
}
