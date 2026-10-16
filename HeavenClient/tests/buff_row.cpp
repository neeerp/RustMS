#include "../IO/Components/BuffRow.h"

#include <cassert>
#include <cmath>
#include <string>

namespace
{
	using Row = ms::BuffRow<std::string>;

	std::string order(const Row& row)
	{
		std::string ids;

		for (auto& entry : row.icons())
			ids += entry.icon;

		return ids;
	}

	bool near(float a, float b)
	{
		return std::fabs(a - b) < 0.001f;
	}
}

int main()
{
	enum Stat { SPEED = 1, JUMP, RECOVERY, WATK };

	// Newest on the right; a recast moves to the right end.
	Row row;
	row.add(1002, SPEED, 120000, [] { return "N"; });
	row.add(1001, RECOVERY, 30000, [] { return "R"; });
	assert(order(row) == "NR");
	row.add(1002, SPEED, 120000, [] { return "n"; });
	assert(order(row) == "Rn");

	// A buff with several stats is one icon, named once per stat.
	row.add(2001, WATK, 60000, [] { return "B"; });
	row.add(2001, JUMP, 60000, [] { return "X"; });
	assert(order(row) == "RnB");
	assert(row.icons().back().stats.size() == 2);

	// Another buff on one of its stats takes that stat; cancelling the rest
	// removes the icon.
	row.add(3001, JUMP, 60000, [] { return "J"; });
	assert(order(row) == "RnBJ");
	row.cancel(WATK);
	assert(order(row) == "RnJ");
	row.add(3002, SPEED, 60000, [] { return "S"; });
	assert(order(row) == "RJS");
	row.cancel(RECOVERY);
	row.cancel(JUMP);
	assert(order(row) == "S");

	// The shadow steps every sixteenth of the duration; the blink starts at 3 s.
	ms::BuffTimer timer(16000);
	assert(timer.shadow() == 15 && near(timer.opacity(), 0xd2 / 255.0f));
	timer.update(1000);
	assert(timer.shadow() == 15);
	timer.update(1);
	assert(timer.shadow() == 14);
	timer.update(11998);
	assert(timer.shadow() == 3 && near(timer.opacity(), 0xd2 / 255.0f));
	timer.update(1);
	assert(timer.shadow() == 3 && near(timer.opacity(), 0xd2 / 255.0f));
	timer.update(1);
	assert(timer.shadow() == 2);
	timer.update(249);
	assert(near(timer.opacity(), (0xd2 + 0x40) / 2.0f / 255.0f));
	timer.update(250);
	assert(near(timer.opacity(), 0xd2 / 255.0f));
	timer.update(3000);
	assert(timer.shadow() == 0);

	// Too short to cross 3 s left: no blink.
	ms::BuffTimer brief(2000);
	brief.update(1500);
	assert(near(brief.opacity(), 0xd2 / 255.0f));

	return 0;
}
