import { createFileRoute } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import type {
  ColumnDef,
  HeaderContext,
  CellContext,
  Row,
  ColumnFiltersState,
} from '@tanstack/react-table'
import {
  flexRender,
  getCoreRowModel,
  useReactTable,
  getPaginationRowModel,
  getFilteredRowModel,
  getSortedRowModel,
} from '@tanstack/react-table'
import { ArrowUpDown, Download } from 'lucide-react'
import * as React from 'react'

import { fetchAllTransactions, exportCsv } from '../lib/api'
import type { Transaction } from '../lib/types'
import { Button } from '../components/ui/button'
import { Input } from '../components/ui/input'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '../components/ui/table'
import {
  Card,
  CardContent,
  CardFooter,
  CardHeader,
} from '../components/ui/card'

export const Route = createFileRoute('/transactions')({
  component: TransactionsPage,
})

const columns: ColumnDef<Transaction>[] = [
  {
    accessorKey: 'time',
    header: ({ column }: HeaderContext<Transaction, unknown>) => {
      return (
        <Button
          variant="ghost"
          onClick={() => column.toggleSorting(column.getIsSorted() === 'asc')}
        >
          Date
          <ArrowUpDown className="ml-2 h-4 w-4" />
        </Button>
      )
    },
    cell: ({ row }: CellContext<Transaction, unknown>) => {
      const timeValue = row.getValue('time')
      const date = new Date(String(timeValue))
      const year = date.getFullYear()
      const month = String(date.getMonth() + 1).padStart(2, '0')
      const day = String(date.getDate()).padStart(2, '0')
      const hours = String(date.getHours()).padStart(2, '0')
      const minutes = String(date.getMinutes()).padStart(2, '0')
      const formattedDate = `${year}-${month}-${day} ${hours}:${minutes}`
      return <div className="font-medium">{formattedDate}</div>
    },
  },
  {
    accessorKey: 'merchant',
    header: 'Merchant',
    cell: ({ row }: CellContext<Transaction, unknown>) => (
      <div>{row.getValue('merchant')}</div>
    ),
  },
  {
    accessorKey: 'amount',
    header: ({ column }: HeaderContext<Transaction, unknown>) => {
      return (
        <Button
          variant="ghost"
          onClick={() => column.toggleSorting(column.getIsSorted() === 'asc')}
          className="text-right w-full justify-end"
        >
          Amount
          <ArrowUpDown className="ml-2 h-4 w-4" />
        </Button>
      )
    },
    cell: ({ row }: CellContext<Transaction, unknown>) => {
      const amount = row.getValue('amount')
      const formatted = amount as number
      return <div className="text-right font-medium">{formatted}</div>
    },
  },
]

function TransactionsPage() {
  const {
    data: transactions,
    isLoading,
    error,
  } = useQuery<Transaction[], Error>({
    queryKey: ['transactions'],
    queryFn: fetchAllTransactions,
  })

  const [columnFilters, setColumnFilters] = React.useState<ColumnFiltersState>(
    [],
  )
  const [isExporting, setIsExporting] = React.useState(false)

  const table = useReactTable<Transaction>({
    data: transactions ?? [],
    columns,
    getCoreRowModel: getCoreRowModel(),
    getPaginationRowModel: getPaginationRowModel(),
    onColumnFiltersChange: setColumnFilters,
    getFilteredRowModel: getFilteredRowModel(),
    getSortedRowModel: getSortedRowModel(),
    state: {
      columnFilters,
    },
    initialState: {
      pagination: {
        pageSize: 15,
      },
      sorting: [
        {
          id: 'time',
          desc: true,
        },
      ],
    },
  })

  const handleExportCsv = async () => {
    try {
      setIsExporting(true)

      // Extract current filters to apply to export
      const merchantFilter = table
        .getColumn('merchant')
        ?.getFilterValue() as string
      // const timeFilter = table.getColumn('time')?.getFilterValue() as string

      const exportParams: any = {}
      if (merchantFilter) {
        exportParams.merchant = merchantFilter
      }

      // For time filter, we might need to parse it depending on how the user enters it
      // For now, we'll export all data matching the merchant filter

      const blob = await exportCsv(exportParams)

      // Create download link
      const url = window.URL.createObjectURL(blob)
      const link = document.createElement('a')
      link.href = url
      link.download = `transactions_export_${new Date().toISOString().split('T')[0]}.csv`
      document.body.appendChild(link)
      link.click()
      document.body.removeChild(link)
      window.URL.revokeObjectURL(url)
    } catch {
      console.error('Export failed:', error)
      alert('Export failed. Please try again.')
    } finally {
      setIsExporting(false)
    }
  }

  if (isLoading) return <div className="p-4">Loading transactions...</div>
  if (error)
    return (
      <div className="p-4 text-red-500">
        Error loading transactions: {error.message}
      </div>
    )

  return (
    <div className="p-4">
      <div className="mb-6">
        <h1 className="text-3xl font-bold tracking-tight mb-2">Transactions</h1>
        <p className="text-muted-foreground">
          View and manage your transaction history.
        </p>
      </div>
      <Card>
        <CardHeader>{/* <CardTitle>Transactions</CardTitle> */}</CardHeader>
        <CardContent>
          <div className="flex items-center justify-between py-4">
            <div className="flex items-center space-x-2">
              <Input
                placeholder="Filter by date..."
                value={table.getColumn('time')?.getFilterValue() as string}
                onChange={(event) =>
                  table.getColumn('time')?.setFilterValue(event.target.value)
                }
                className="max-w-sm"
              />
              <Input
                placeholder="Filter by merchant..."
                value={table.getColumn('merchant')?.getFilterValue() as string}
                onChange={(event) =>
                  table
                    .getColumn('merchant')
                    ?.setFilterValue(event.target.value)
                }
                className="max-w-sm"
              />
            </div>
            <Button
              onClick={handleExportCsv}
              disabled={isExporting}
              variant="outline"
              size="sm"
            >
              <Download className="mr-2 h-4 w-4" />
              {isExporting ? 'Exporting...' : 'Export CSV'}
            </Button>
          </div>
          <div className="rounded-md border">
            <Table>
              <TableHeader>
                {table.getHeaderGroups().map((headerGroup) => (
                  <TableRow key={headerGroup.id}>
                    {headerGroup.headers.map((header) => (
                      <TableHead
                        key={header.id}
                        className={
                          header.column.id === 'amount' ? 'text-right' : ''
                        }
                      >
                        {header.isPlaceholder
                          ? null
                          : flexRender(
                              header.column.columnDef.header,
                              header.getContext(),
                            )}
                      </TableHead>
                    ))}
                  </TableRow>
                ))}
              </TableHeader>
              <TableBody>
                {table.getRowModel().rows.length ? (
                  table.getRowModel().rows.map((row: Row<Transaction>) => (
                    <TableRow
                      key={row.id}
                      data-state={row.getIsSelected() && 'selected'}
                    >
                      {row.getVisibleCells().map((cell) => (
                        <TableCell
                          key={cell.id}
                          className={
                            cell.column.id === 'amount' ? 'text-right' : ''
                          }
                        >
                          {flexRender(
                            cell.column.columnDef.cell,
                            cell.getContext(),
                          )}
                        </TableCell>
                      ))}
                    </TableRow>
                  ))
                ) : (
                  <TableRow>
                    <TableCell
                      colSpan={columns.length}
                      className="h-24 text-center"
                    >
                      No results.
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </div>
        </CardContent>
        <CardFooter>
          <div className="flex items-center justify-end w-full space-x-2 py-4">
            <Button
              variant="outline"
              size="sm"
              onClick={() => table.previousPage()}
              disabled={!table.getCanPreviousPage()}
            >
              Previous
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => table.nextPage()}
              disabled={!table.getCanNextPage()}
            >
              Next
            </Button>
          </div>
        </CardFooter>
      </Card>
    </div>
  )
}
