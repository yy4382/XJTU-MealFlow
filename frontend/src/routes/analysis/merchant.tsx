import { createFileRoute } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { Bar, BarChart, CartesianGrid, XAxis, YAxis, Tooltip } from 'recharts'

import { fetchAllTransactions } from '../../lib/api'
import type { Transaction } from '../../lib/types'
import { processMerchantData } from '../../lib/analysis-utils'
import { ChartContainer, ChartTooltipContent } from '../../components/ui/chart'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '../../components/ui/card'

export const Route = createFileRoute('/analysis/merchant')({
  component: MerchantAnalysisPage,
})

// Chart config can be dynamic if needed, or simplified if colors are uniform
// For simplicity, we might not need a complex config here if all bars are the same color
const chartConfig = {
  spending: {
    label: 'Spending (Abs)', // Label for legend/tooltip
    color: 'hsl(var(--chart-1))', // Default color for bars
  },
} satisfies Record<string, { label: string; color: string }>

function MerchantAnalysisPage() {
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
    ? processMerchantData(transactions)
    : { chartData: [], rawData: [] }

  if (isLoading) return <div className="p-4">Loading chart data...</div>
  if (error)
    return (
      <div className="p-4 text-red-500">
        Error loading data: {error.message}
      </div>
    )

  // Check if chartData is empty
  if (analysisResult.chartData.length === 0) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Merchant Spending Analysis</CardTitle>
          <CardDescription>Breakdown of spending by merchant.</CardDescription>
        </CardHeader>
        <CardContent className="h-[400px] flex items-center justify-center">
          <p>No transaction data available to display merchant spending.</p>
        </CardContent>
      </Card>
    )
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Top Merchant Spending</CardTitle>
        <CardDescription>
          Absolute spending amounts for top merchants.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <ChartContainer config={chartConfig} className="min-h-[400px] w-full">
          {/* Using a taller chart for potentially many merchant names */}
          <BarChart
            accessibilityLayer
            data={analysisResult.chartData}
            layout="vertical"
          >
            <CartesianGrid horizontal={false} />
            <XAxis type="number" dataKey="value" />
            <YAxis
              dataKey="name" // Merchant name
              type="category"
              tickLine={false}
              axisLine={false}
              tickMargin={5}
              width={150} // Adjust width for Y-axis labels if names are long
              interval={0} // Show all labels
            />
            <Tooltip
              cursor={{ fill: 'hsl(var(--muted))' }}
              content={<ChartTooltipContent hideLabel />}
            />
            <Bar dataKey="value" fill={chartConfig.spending.color} radius={5} />
          </BarChart>
        </ChartContainer>
      </CardContent>
    </Card>
  )
}
