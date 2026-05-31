# Golden fixture for verifying that the bundled mruby compiler (mrbc) emits the
# expected RITE bytecode. The committed bytecode_golden.mrb is regenerated only
# when the mruby submodule is updated; see tests/bytecode_golden.rs.
#
# The program is deliberately broad so that a miscompiled mrbc is likely to
# deviate from the golden output: method definition, recursion, a ternary,
# integer arithmetic and comparison, an array literal, a block iterator,
# compound assignment, string interpolation, and a builtin call.
def fib(n)
  n < 2 ? n : fib(n - 1) + fib(n - 2)
end

total = 0
[1, 2, 3, 4, 5, 6, 7, 8, 9, 10].each do |i|
  total += fib(i)
end

puts "fib_sum=#{total}"
