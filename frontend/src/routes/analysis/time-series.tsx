import { createFileRoute } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import {
  Line,
  LineChart,
  CartesianGrid,
  XAxis,
  YAxis,
  Tooltip,
  Legend,
} from 'recharts'

import { fetchAllTransactions } from '../../lib/api'
import type { Transaction } from '../../lib/types'
import { processTimeSeriesData } from '../../lib/analysis-utils'
import {
  ChartContainer,
  ChartTooltipContent,
  ChartLegend,
} from '../../components/ui/chart'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '../../components/ui/card'

export const Route = createFileRoute('/analysis/time-series')({
  component: TimeSeriesAnalysisPage,
})

const chartConfig = {
  value: {
    label: 'Spending',
    color: 'hsl(var(--chart-1))',
  },
} satisfies Record<string, { label: string; color: string }>

function TimeSeriesAnalysisPage() {
  const {
    data: transactions,
    isLoading,
    error,
  } = useQuery<Transaction[], Error>({
    queryKey: ['transactions'],
    queryFn: fetchAllTransactions,
    staleTime: 1000 * 60 * 5, // Cache for 5 minutes
  })

  const analysisResult = transactions
    ? processTimeSeriesData(transactions)
    : { chartData: [] }

  if (isLoading) return <div className="p-4">Loading chart data...</div>
  if (error)
    return (
      <div className="p-4 text-red-500">
        Error loading data: {error.message}
      </div>
    )

  if (analysisResult.chartData.length === 0) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Time Series Analysis</CardTitle>
          <CardDescription>Spending trends over time.</CardDescription>
        </CardHeader>
        <CardContent className="h-[400px] flex items-center justify-center">
          <p>No transaction data available to display time series.</p>
        </CardContent>
      </Card>
    )
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Time Series Analysis</CardTitle>
        <CardDescription>Monthly spending totals over time.</CardDescription>
      </CardHeader>
      <CardContent>
        <ChartContainer config={chartConfig} className="min-h-[300px] w-full">
          <LineChart
            accessibilityLayer
            data={analysisResult.chartData}
            margin={{
              top: 5,
              right: 10,
              left: 10,
              bottom: 5,
            }}
          >
            <CartesianGrid vertical={false} strokeDasharray="3 3" />
            <XAxis
              dataKey="name" // "YYYY-MM"
              tickLine={false}
              axisLine={false}
              tickMargin={8}
              // tickFormatter={(value) => value.slice(-2)} // Optionally format to show only month
            />
            <YAxis tickLine={false} axisLine={false} tickMargin={8} />
            <Tooltip
              cursor={false}
              content={<ChartTooltipContent indicator="line" hideLabel />}
            />
            <Legend content={<ChartLegend />} />
            <Line
              dataKey="value"
              type="monotone"
              strokeWidth={2}
              dot={true}
              stroke={chartConfig.value.color}
            />
          </LineChart>
        </ChartContainer>
      </CardContent>
    </Card>
  )
}
