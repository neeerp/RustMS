#include "../Character/GachaExp.h"

#include <cassert>

int main()
{
	using ms::gacha_exp::WritRefusal;
	using ms::gacha_exp::writ_refusal;

	// A writ is for its maxLevel and under, and only once the last is taken.
	assert(writ_refusal(50, 50, 0) == WritRefusal::NONE);
	assert(writ_refusal(51, 50, 0) == WritRefusal::LEVEL);
	assert(writ_refusal(51, 50, 100) == WritRefusal::LEVEL);
	assert(writ_refusal(10, 50, 100) == WritRefusal::EXP_LEFT);
	assert(writ_refusal(120, 0, 0) == WritRefusal::NONE);

	// Taking it in is offered up to level 50, with some left.
	assert(ms::gacha_exp::may_take(50, 1));
	assert(!ms::gacha_exp::may_take(51, 1));
	assert(!ms::gacha_exp::may_take(10, 0));

	assert(ms::gacha_exp::tooltip(5, 15, 0) == "EXP : 5 / 15");
	assert(ms::gacha_exp::tooltip(5, 15, 51) == "EXP : 5 / 15  GachaponEXP : 51");
}
