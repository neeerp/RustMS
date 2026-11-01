#include "../Character/AutoAssign.h"

#include <cassert>
#include <vector>

using namespace ms;
namespace aa = auto_assign;

namespace
{
	// What goes on the wire for a job, level, totals and AP: the report's worked examples.
	std::vector<aa::Pair> sent(int32_t job, int32_t level, aa::Totals have, int32_t ap, bool brawler = true)
	{
		std::vector<aa::Pair> pairs = aa::distribute(job, level, have, brawler);
		aa::spread(pairs, ap);

		return pairs;
	}

	bool same(const std::vector<aa::Pair> &pairs, std::vector<aa::Pair> want)
	{
		if (pairs.size() != want.size())
			return false;

		for (size_t i = 0; i < pairs.size(); i++)
			if (pairs[i].flag != want[i].flag || pairs[i].amount != want[i].amount)
				return false;

		return true;
	}

	void test_branches()
	{
		// A warrior short of DEX takes it all there.
		assert(same(sent(110, 30, { 95, 55, 4, 4 }, 5), { { aa::DEX, 5 }, { aa::STR, 0 } }));
		// Already over both targets, the rest goes to the second, STR.
		assert(same(sent(110, 30, { 110, 62, 4, 4 }, 5), { { aa::DEX, 0 }, { aa::STR, 5 } }));
		assert(same(sent(110, 31, { 100, 58, 4, 4 }, 5), { { aa::DEX, 3 }, { aa::STR, 2 } }));
		assert(same(sent(200, 30, { 4, 4, 120, 30 }, 5), { { aa::LUK, 3 }, { aa::INT, 2 } }));
		assert(same(sent(410, 50, { 4, 90, 4, 160 }, 10), { { aa::DEX, 0 }, { aa::LUK, 10 } }));
		assert(same(sent(310, 30, { 30, 120, 4, 4 }, 5), { { aa::STR, 5 }, { aa::DEX, 0 } }));
		assert(same(sent(320, 30, { 30, 120, 4, 4 }, 5), { { aa::STR, 0 }, { aa::DEX, 5 } }));
		assert(same(sent(520, 30, { 25, 120, 4, 4 }, 5), { { aa::STR, 5 }, { aa::DEX, 0 } }));
		assert(same(sent(510, 30, { 100, 48, 4, 4 }, 5), { { aa::DEX, 2 }, { aa::STR, 3 } }));
		// The first-job Pirate's two halves.
		assert(same(sent(500, 20, { 60, 35, 4, 4 }, 5, true), { { aa::DEX, 5 }, { aa::STR, 0 } }));
		assert(same(sent(500, 20, { 15, 80, 4, 4 }, 5, false), { { aa::STR, 5 }, { aa::DEX, 0 } }));
		// A Thunder Breaker is a brawler either way.
		assert(same(sent(1500, 20, { 60, 35, 4, 4 }, 5, false), { { aa::DEX, 5 }, { aa::STR, 0 } }));
		assert(same(sent(0, 11, { 50, 8, 4, 4 }, 5), { { aa::DEX, 3 }, { aa::STR, 2 } }));
		// Cygnus and Aran go by job % 1000.
		assert(same(sent(2110, 31, { 100, 58, 4, 4 }, 5), { { aa::DEX, 3 }, { aa::STR, 2 } }));
	}

	void test_unbuilt_jobs()
	{
		assert(aa::distribute(900, 30, { 4, 4, 4, 4 }, true).empty());
		assert(aa::distribute(301, 30, { 4, 4, 4, 4 }, true).empty());
		assert(aa::distribute(530, 30, { 4, 4, 4, 4 }, true).empty());
		// 2100 is the Aran's first job, a warrior; 1001 is no job.
		assert(!aa::distribute(2100, 30, { 4, 4, 4, 4 }, true).empty());
		assert(aa::distribute(1001, 30, { 4, 4, 4, 4 }, true).empty());
	}

	void test_buttons()
	{
		assert(aa::no_buttons(0, 10) && aa::no_buttons(2001, 1) && !aa::no_buttons(0, 11) && !aa::no_buttons(100, 10));
		assert(aa::shown(110, 0) && !aa::shown(110, 1) && !aa::shown(110, 2));
		assert(!aa::shown(500, 0) && aa::shown(500, 1) && aa::shown(500, 2));
		// Raw job: a Thunder Breaker gets the one button.
		assert(aa::shown(1500, 0) && !aa::shown(1500, 1));
		assert(!aa::shown(900, 0) && !aa::shown(910, 1));
	}

	void test_message()
	{
		std::string text = aa::message({ { aa::DEX, 3 }, { aa::STR, 2 } });

		assert(text == "    Your AP will be distributed as follows:\\n\\n"
					   "     STR : +2\\n     DEX : +3\\n     INT : +0\\n     LUK : +0\\n");
	}
}

int main()
{
	test_branches();
	test_unbuilt_jobs();
	test_buttons();
	test_message();
}
