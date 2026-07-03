EXTENDS Integers, Sequences, FiniteSets

RECURSIVE Sum(_)
Sum(f) == IF DOMAIN f = {} THEN 0
          ELSE LET x == CHOOSE x \in DOMAIN f : TRUE
               IN f[x] + Sum([y \in (DOMAIN f \ {x}) |-> f[y]])

CONSTANT Users,          \* ユーザー集合
         SystemAccount,  \* システム（手数料）アカウント
         MaxBalance,     \* モデル検査用の最大残高
         FeeRateNum,     \* 手数料分子 (例: 5)
         FeeRateDenom    \* 手数料分母 (例: 100)

VARIABLES balances,      \* Actors -> Coins
          points,        \* Users -> Points
          daily_spent,   \* Users -> Coins spent today
          minted         \* SurpriseBonus 等で発行された累計コイン

Actors == Users \cup {SystemAccount}

TypeOK == 
    /\ balances \in [Actors -> 0..MaxBalance]
    /\ points \in [Users -> 0..MaxBalance]
    /\ daily_spent \in [Users -> 0..MaxBalance]
    /\ minted \in 0..MaxBalance

Init == 
    /\ balances = [a \in Actors |-> IF a = SystemAccount THEN 0 ELSE 100]
    /\ points = [u \in Users |-> 0]
    /\ daily_spent = [u \in Users |-> 0]
    /\ minted = 0

\* 取引アクション: 購入
BuyItem(buyer, seller, cost, daily_limit) ==
    /\ buyer \in Users
    /\ seller \in Users
    /\ buyer /= seller
    /\ cost > 0
    /\ balances[buyer] >= cost
    /\ daily_spent[buyer] + cost <= daily_limit
    /\ LET fee == (cost * FeeRateNum) \div FeeRateDenom
           creator_cut == cost - fee
           points_earned == cost \div 10 \* 10% 還元例
       IN
        /\ balances' = [balances EXCEPT ![buyer] = balances[buyer] - cost,
                                        ![seller] = balances[seller] + creator_cut,
                                        ![SystemAccount] = balances[SystemAccount] + fee]
        /\ points' = [points EXCEPT ![seller] = points[seller] + points_earned]
        /\ daily_spent' = [daily_spent EXCEPT ![buyer] = daily_spent[buyer] + cost]
        /\ UNCHANGED minted

\* 制限リセットアクション
ResetDailyLimits ==
    /\ daily_spent' = [u \in Users |-> 0]
    /\ UNCHANGED <<balances, points, minted>>

\* A2C サプライズボーナス（mint）
SurpriseBonus(recipient, bonus, max_daily_bonus, daily_bonus_issued) ==
    /\ recipient \in Users
    /\ bonus > 0
    /\ daily_bonus_issued + bonus <= max_daily_bonus
    /\ balances' = [balances EXCEPT ![recipient] = balances[recipient] + bonus]
    /\ minted' = minted + bonus
    /\ UNCHANGED <<points, daily_spent>>

Next == 
    \/ \E b, s \in Users, c \in 1..20 : BuyItem(b, s, c, 50)
    \/ ResetDailyLimits
    \/ \E r \in Users, bonus \in 1..10, issued \in 0..500, max \in 500 : SurpriseBonus(r, bonus, max, issued)

Spec == Init /\ [][Next]_<<balances, points, daily_spent, minted>>

\* 不変条件: 初期配布 + mint 分を超えない（保存則）
TotalCoins == Sum(balances)
CoinsConserved == TotalCoins = (Cardinality(Users) * 100) + minted

-----------------------------------------------------------------------------
=============================================================================
