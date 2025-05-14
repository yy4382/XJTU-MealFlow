import { createFileRoute, Outlet, Link } from '@tanstack/react-router'
import { cn } from '../lib/utils' // For conditional classnames

export const Route = createFileRoute('/analysis')({
  component: AnalysisLayout,
})

const analysisNavLinks = [
  { name: 'Time Period', to: '/analysis/time-period' },
  { name: 'Time Series', to: '/analysis/time-series' },
  { name: 'By Merchant', to: '/analysis/merchant' },
]

function AnalysisLayout() {
  return (
    <div className="p-4">
      <div className="mb-6">
        <h1 className="text-3xl font-bold tracking-tight mb-2">
          Spending Analysis
        </h1>
        <p className="text-muted-foreground">
          Explore your spending habits through various analytical lenses.
        </p>
      </div>

      <div className="flex flex-col md:flex-row gap-6">
        <nav className="flex flex-col md:w-1/5 space-y-1">
          {analysisNavLinks.map((link) => (
            <Link
              key={link.name}
              to={link.to}
              // activeOptions={{ exact: link.exact ?? false }}
              className={cn(
                'group flex items-center rounded-md px-3 py-2 text-sm font-medium',
                'text-muted-foreground hover:bg-accent hover:text-accent-foreground',
              )}
              activeProps={{
                className:
                  'bg-primary text-primary-foreground hover:bg-primary/90 hover:text-primary-foreground',
              }}
            >
              {link.name}
            </Link>
          ))}
        </nav>
        <div className="flex-1 md:w-4/5">
          <Outlet />
        </div>
      </div>
    </div>
  )
}
