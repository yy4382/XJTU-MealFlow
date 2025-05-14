import { Outlet, createRootRouteWithContext } from '@tanstack/react-router'
import { TanStackRouterDevtools } from '@tanstack/react-router-devtools'
import { Sidebar } from '../components/Sidebar'

import TanStackQueryLayout from '../integrations/tanstack-query/layout.tsx'

import type { QueryClient } from '@tanstack/react-query'
import { SidebarProvider, SidebarTrigger } from '@/components/ui/sidebar.tsx'

interface MyRouterContext {
  queryClient: QueryClient
}

export const Route = createRootRouteWithContext<MyRouterContext>()({
  component: RootComponent,
})

function RootComponent() {
  return (
    <div className="flex h-screen bg-gray-100">
      <SidebarProvider>
        <Sidebar />
        <main className="flex-1 flex flex-col overflow-hidden relative">
          <SidebarTrigger className="absolute top-2 left-2" />
          <div className="flex-1 overflow-x-hidden overflow-y-auto p-6">
            <Outlet />
          </div>
        </main>
        <TanStackRouterDevtools position="bottom-right" />
        <TanStackQueryLayout />
      </SidebarProvider>
    </div>
  )
}
