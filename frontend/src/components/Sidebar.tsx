import { Link } from '@tanstack/react-router'
import { Home, ListTree, BarChart2, Settings } from 'lucide-react' // Import icons
import {
  Sidebar as ShadcnSidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuItem,
} from '@/components/ui/sidebar' // Assuming this is the correct path for shadcn sidebar

const navigation = [
  { name: 'Transactions', href: '/transactions', icon: ListTree },
  { name: 'Analysis', href: '/analysis', icon: BarChart2 },
  { name: 'Settings', href: '/settings', icon: Settings },
]

export function Sidebar() {
  return (
    <ShadcnSidebar className="w-64 bg-gray-800 text-white">
      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupLabel className="p-4 border-b border-gray-700">
            <Link to="/" className="flex items-center space-x-2">
              <Home className="h-8 w-8 text-sky-400" />
              <h1 className="text-2xl font-semibold">MealFlow</h1>
            </Link>
          </SidebarGroupLabel>
          <SidebarMenu className="flex-grow p-4 space-y-2">
            {navigation.map((item) => (
              <SidebarMenuItem key={item.name}>
                <Link
                  to={item.href}
                  className="flex items-center px-3 py-2 text-gray-300 rounded-md hover:bg-gray-700 hover:text-white transition-colors duration-150 ease-in-out group"
                  activeProps={{
                    className: 'bg-sky-500 text-white',
                  }}
                  inactiveProps={{
                    className: 'hover:bg-gray-700 hover:text-white',
                  }}
                >
                  <item.icon
                    className="mr-3 h-5 w-5 text-gray-400 group-hover:text-gray-300 transition-colors duration-150 ease-in-out"
                    aria-hidden="true"
                  />
                  {item.name}
                </Link>
              </SidebarMenuItem>
            ))}
          </SidebarMenu>
        </SidebarGroup>
      </SidebarContent>
      <div className="p-4 border-t border-gray-700">
        {/* Footer content for sidebar if any */}
        <p className="text-xs text-gray-500">© 2024 MealFlow</p>
      </div>
    </ShadcnSidebar>
  )
}
