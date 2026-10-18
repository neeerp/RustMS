#include "../IO/Components/FadeNotice.h"

#include <cassert>
#include <vector>

int main()
{
	// Every notice stacks above the ones made before it; when one goes, the
	// later ones move down, and moving a notice keeps its place.
	{
		std::vector<ms::FadeNotice> line;
		line.emplace_back(1000);
		line.emplace_back(1000);
		line.emplace_back(1000);
		ms::FadeNotice other(1000);
		assert(line[0].raised() == 0 && line[2].raised() == 10 && other.raised() == 15);

		line.erase(line.begin() + 1);
		assert(line[0].raised() == 0 && line[1].raised() == 5 && other.raised() == 10);

		line.erase(line.begin());
		assert(line[0].raised() == 0 && other.raised() == 5);
	}

	assert(ms::FadeNotice(1000).raised() == 0);

	// Untouched: a second in, until 30 s from being made, a second out.
	ms::FadeNotice notice(30000);
	assert(notice.opacity() == 0.0f);
	notice.update(500);
	assert(notice.opacity() == 0.5f);
	notice.update(501);
	assert(notice.opacity() == 1.0f && !notice.closed());
	notice.update(28999);
	assert(notice.opacity() == 1.0f && !notice.closed());
	notice.update(1);
	assert(notice.closed() && notice.opacity() == 1.0f);
	notice.update(500);
	assert(notice.opacity() == 0.5f && !notice.gone());
	notice.update(501);
	assert(notice.gone() && notice.opacity() == 0.0f);

	// Closed while showing: fades out at once.
	ms::FadeNotice pressed(30000);
	pressed.update(2000);
	pressed.close();
	assert(pressed.closed());
	pressed.update(8);
	assert(pressed.opacity() == 1.0f);
	pressed.update(1001);
	assert(pressed.gone());

	// Closed while fading in: finishes fading in first.
	ms::FadeNotice early(30000);
	early.update(200);
	early.close();
	early.update(300);
	assert(early.opacity() == 0.5f && !early.gone());
	early.update(501);
	early.update(8);
	early.update(1001);
	assert(early.gone());

	return 0;
}
