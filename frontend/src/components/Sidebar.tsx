import { Link } from '@tanstack/react-router'
import { Home, ListTree, BarChart2, Settings } from 'lucide-react'
import {
  Sidebar as ShadcnSidebar,
  SidebarContent,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuItem,
  SidebarFooter,
} from '@/components/ui/sidebar'

const navigation = [
  { name: 'Transactions', href: '/transactions', icon: ListTree },
  { name: 'Analysis', href: '/analysis', icon: BarChart2 },
  { name: 'Settings', href: '/settings', icon: Settings },
]

export function Sidebar() {
  return (
    <ShadcnSidebar className="w-64">
      <SidebarHeader className="p-4 border-b">
        <Link to="/" className="flex items-center space-x-2">
          <Home className="h-8 w-8" />
          <h1 className="text-2xl font-semibold">MealFlow</h1>
        </Link>
      </SidebarHeader>
      <SidebarContent className="p-0">
        <SidebarMenu className="p-4 space-y-1">
          {navigation.map((item) => {
            // const isActive = !!matchRoute({ to: item.href, fuzzy: true }) // isActive prop caused type error
            return (
              <SidebarMenuItem key={item.name}>
                <Link
                  to={item.href}
                  className="flex items-center w-full space-x-2 px-2 py-1.5 rounded-md"
                >
                  <item.icon className="mr-2 h-4 w-4" aria-hidden="true" />
                  {item.name}
                </Link>
              </SidebarMenuItem>
            )
          })}
        </SidebarMenu>
      </SidebarContent>
      <SidebarFooter className="p-4 border-t">
        <p className="text-xs">© 2025 MealFlow</p>
      </SidebarFooter>
    </ShadcnSidebar>
  )
}
