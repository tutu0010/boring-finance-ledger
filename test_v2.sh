#!/bin/bash

echo "===== Creating Test Data ====="

cargo run -- income 15000 father semester_fees
cargo run -- income 2500 scholarship merit_award
cargo run -- income 800 freelance rust_project

cargo run -- expense 120 food burger_king
cargo run -- expense 350 food kfc_dinner
cargo run -- expense 80 transport auto_to_station
cargo run -- expense 1200 education textbooks
cargo run -- expense 500 entertainment movie_night
cargo run -- expense 150 food shawarma

cargo run -- lend 500 alex lunch_and_snacks
cargo run -- lend 200 Rahul cab_share

cargo run -- borrow 1000 max emergency_cash
cargo run -- borrow 300 Amal dinner

cargo run -- receive 200 alex
cargo run -- receive 50 Rahul

cargo run -- repay 400 max
cargo run -- repay 100 Amal

cargo run -- subscribe 89.9 spotify monthly
cargo run -- subscribe 130 google_one monthly
cargo run -- subscribe 799 github_copilot monthly

echo ""
echo "===== HISTORY ====="
cargo run -- history

echo ""
echo "===== SUMMARY ====="
cargo run -- summary

echo ""
echo "===== OWED ====="
cargo run -- owed

echo ""
echo "===== DEBTS ====="
cargo run -- debts

echo ""
echo "===== LIST EXPENSES ====="
cargo run -- list expenses

echo ""
echo "===== LIST INCOME ====="
cargo run -- list income

echo ""
echo "===== LIST LOANS ====="
cargo run -- list loans

echo ""
echo "===== LIST SUBSCRIPTIONS ====="
cargo run -- list subscriptions

echo ""
echo "===== FIND alex ====="
cargo run -- find alex

echo ""
echo "===== FIND max ====="
cargo run -- find max

echo ""
echo "===== FIND BURGER ====="
cargo run -- find burger

echo ""
echo "===== FIND FOOD ====="
cargo run -- find food

echo ""
echo "===== FIND SPOTIFY ====="
cargo run -- find spotify

echo ""
echo "===== DATE RANGE ====="
cargo run -- summary 2026-01 2026-12

echo ""
echo "===== UNDO TEST ====="

cargo run -- expense 999 misc accidental_test

echo ""
echo "Before Undo:"
cargo run -- find accidental

cargo run -- undo

echo ""
echo "After Undo:"
cargo run -- find accidental

echo ""
echo "===== FINAL SUMMARY ====="
cargo run -- summary

echo ""
echo "===== TESTS ====="
cargo test

echo ""
echo "===== DONE ====="
