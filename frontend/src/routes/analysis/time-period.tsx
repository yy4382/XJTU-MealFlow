import { createFileRoute } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import {
  Bar,
  BarChart,
  CartesianGrid,
  XAxis,
  YAxis,
  Tooltip,
  Cell,
} from 'recharts' // Directly use recharts for more control if shadcn/ui chart is a wrapper

import { fetchAllTransactions } from '../../lib/api'
import type { Transaction } from '../../lib/types'
import { processTimePeriodData } from '../../lib/analysis-utils'
import { ChartContainer, ChartTooltipContent } from '../../components/ui/chart' // Use Shadcn chart components
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '../../components/ui/card'
import { useMemo } from 'react'

export const Route = createFileRoute('/analysis/time-period')({
  component: TimePeriodAnalysisPage,
})

const chartConfig = {
  breakfast: {
    label: 'Breakfast',
    color: 'hsl(var(--chart-1))',
  },
  lunch: {
    label: 'Lunch',
    color: 'hsl(var(--chart-2))',
  },
  dinner: {
    label: 'Dinner',
    color: 'hsl(var(--chart-3))',
  },
  other: {
    label: 'Other',
    color: 'hsl(var(--chart-4))',
  },
} satisfies Record<string, { label: string; color: string }>

function TimePeriodAnalysisPage() {
  const {
    data: transactions,
    isLoading,
    error,
  } = useQuery<Transaction[], Error>({
    queryKey: ['transactions'],
    queryFn: fetchAllTransactions,
    staleTime: 1000 * 60 * 5, // Cache for 5 minutes
  })

  const analysisData = useMemo(() => {
    console.log('processing data')
    return transactions ? processTimePeriodData(transactions) : null
  }, [transactions])
  console.log(analysisData)

  if (isLoading) return <div className="p-4">Loading chart data...</div>
  if (error)
    return (
      <div className="p-4 text-red-500">
        Error loading data: {error.message}
      </div>
    )
  if (!analysisData) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Time Period Analysis</CardTitle>
          <CardDescription>
            Spending patterns across different time periods.
          </CardDescription>
        </CardHeader>
        <CardContent className="h-[400px] flex items-center justify-center">
          <p>
            No transaction data available for the selected period or criteria.
          </p>
        </CardContent>
      </Card>
    )
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Time Period Analysis</CardTitle>
        <CardDescription>
          Transaction counts for breakfast, lunch, dinner, and other times.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <ChartContainer config={chartConfig} className="min-h-[300px] w-full">
          <BarChart
            accessibilityLayer
            data={analysisData.chartData}
            layout="vertical"
          >
            <CartesianGrid horizontal={false} />
            <XAxis type="number" dataKey="value" />
            <YAxis
              dataKey="name"
              type="category"
              tickLine={false}
              axisLine={false}
              tickMargin={8}
            />
            <Tooltip
              cursor={{ fill: 'hsl(var(--muted))' }}
              content={<ChartTooltipContent hideLabel />}
            />
            <Bar dataKey="value" radius={5}>
              {analysisData.chartData.map((entry, index) => {
                const configKey = (
                  entry.name ? entry.name.toLowerCase() : 'unknown'
                ) as keyof typeof chartConfig
                console.log(configKey)
                const color =
                  chartConfig[configKey].color || 'hsl(var(--muted-foreground))'
                return <Cell key={`cell-${index}`} fill={color} />
              })}
            </Bar>
          </BarChart>
        </ChartContainer>
      </CardContent>
      <CardFooter className="flex-col items-start gap-2 text-sm">
        <div className="flex gap-2 font-medium leading-none">
          Breakfast: {analysisData.breakfast}, Lunch: {analysisData.lunch},
          Dinner: {analysisData.dinner}, Other: {analysisData.unknown}
        </div>
      </CardFooter>
    </Card>
  )
}
