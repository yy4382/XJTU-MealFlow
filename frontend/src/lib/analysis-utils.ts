import type { Transaction, ChartData } from './types'

export interface ProcessedTimePeriodData {
  breakfast: number
  lunch: number
  dinner: number
  unknown: number
  chartData: ChartData[] // For direct use with Recharts/Shadcn Charts
}

export interface ProcessedMerchantData {
  chartData: ChartData[] // Expects { name: string, value: number (absolute for chart) }[]
  rawData: { merchant: string; totalAmount: number }[] // Original aggregated amounts
}

/**
 * Checks if a given Date object's time falls within a specified time range.
 * @param date The Date object to check.
 * @param startHour The start hour of the range (0-23).
 * @param startMinute The start minute of the range (0-59).
 * @param endHour The end hour of the range (0-23).
 * @param endMinute The end minute of the range (0-59).
 * @returns True if the time is within the range, false otherwise.
 */
function isTimeInRange(
  date: Date,
  startHour: number,
  startMinute: number,
  endHour: number,
  endMinute: number,
): boolean {
  const hours = date.getHours()
  const minutes = date.getMinutes()

  const currentTimeInMinutes = hours * 60 + minutes
  const startTimeInMinutes = startHour * 60 + startMinute
  const endTimeInMinutes = endHour * 60 + endMinute

  return (
    currentTimeInMinutes >= startTimeInMinutes &&
    currentTimeInMinutes < endTimeInMinutes
  )
}

export function processTimePeriodData(
  transactions: Transaction[],
): ProcessedTimePeriodData {
  const counts = transactions.reduce(
    (acc, transaction) => {
      const transactionTime = new Date(transaction.time) // Assuming transaction.time is an ISO string

      if (isTimeInRange(transactionTime, 5, 0, 10, 30)) {
        // Breakfast: 05:00 - 10:30
        acc.breakfast += 1
      } else if (isTimeInRange(transactionTime, 10, 30, 13, 30)) {
        // Lunch: 10:30 - 13:30
        acc.lunch += 1
      } else if (isTimeInRange(transactionTime, 16, 30, 19, 30)) {
        // Dinner: 16:30 - 19:30
        acc.dinner += 1
      } else {
        acc.unknown += 1
      }
      return acc
    },
    { breakfast: 0, lunch: 0, dinner: 0, unknown: 0 },
  )

  return {
    ...counts,
    chartData: [
      {
        name: 'Breakfast',
        value: counts.breakfast,
        fill: 'hsl(var(--chart-1))',
      }, // Example fill colors
      { name: 'Lunch', value: counts.lunch, fill: 'hsl(var(--chart-2))' },
      { name: 'Dinner', value: counts.dinner, fill: 'hsl(var(--chart-3))' },
      { name: 'Other', value: counts.unknown, fill: 'hsl(var(--chart-4))' },
    ],
  }
}

// Placeholder for other analysis functions
export function processTimeSeriesData(transactions: Transaction[]) {
  // TODO: Implement logic based on src/page/analysis/time_series.rs
  console.log(
    'Processing time series data for:',
    transactions.length,
    'transactions',
  )
  return { chartData: [] }
}

export function processMerchantData(
  transactions: Transaction[],
  topN: number = 15, // Optionally show only top N merchants by spending
): ProcessedMerchantData {
  if (transactions.length === 0) {
    return { chartData: [], rawData: [] }
  }

  const merchantSpending: Map<string, number> = new Map()

  transactions.forEach((transaction) => {
    const currentAmount = merchantSpending.get(transaction.merchant) || 0
    merchantSpending.set(
      transaction.merchant,
      currentAmount + transaction.amount,
    )
  })

  const aggregatedData = Array.from(merchantSpending.entries()).map(
    ([merchant, totalAmount]) => ({
      merchant,
      totalAmount,
    }),
  )

  // Sort by totalAmount (negative for spending, so ascending sort means most spent first)
  // Or sort by Math.abs(totalAmount) descending for top spenders regardless of refunds.
  // The Rust code sorts by actual amount (a.1.total_cmp(b.1)), so smaller (more negative) values come first.
  aggregatedData.sort((a, b) => a.totalAmount - b.totalAmount)

  const rawData = aggregatedData.slice(0, topN) // Take top N or all if less than N

  const chartData: ChartData[] = rawData.map((item) => ({
    name: item.merchant,
    value: Math.abs(item.totalAmount), // Use absolute value for bar chart length
    // We could add originalValue: item.totalAmount here if needed for tooltips
  }))

  return { chartData, rawData }
}

export function processMerchantCategoryData(transactions: Transaction[]) {
  // TODO: Implement logic based on src/page/analysis/merchant_type.rs
  // Note: Merchant category might require some predefined mapping or regex if not directly in data.
  console.log(
    'Processing merchant category data for:',
    transactions.length,
    'transactions',
  )
  return { chartData: [] }
}
